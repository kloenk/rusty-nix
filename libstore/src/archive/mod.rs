use byteorder::{ByteOrder, LittleEndian};
use tokio::io::{AsyncRead, AsyncReadExt};

use futures::future::LocalFutureObj;
use std::boxed::Box;

pub use crate::error::NarError;
use crate::Store;

pub use crate::store::Hash;

use log::*;

use std::io;
use std::sync::Mutex;

pub const NAR_VERSION_MAGIC_1: &'static str = "nix-archive-1";

/// Returned as succesfully parsed nar archive
pub struct NarResult {
    pub hash: Hash,
    pub len: u64,
}

pub struct NarParser<'a, T: ?Sized + AsyncRead + Unpin> {
    reader: Mutex<&'a mut T>,

    store: Mutex<&'a mut Box<dyn Store>>,

    hasher: Mutex<Option<(ring::digest::Context, u64)>>,

    pub base_path: String,
}

impl<'a, T: ?Sized + AsyncRead + Unpin> NarParser<'a, T> {
    pub fn new(base_path: &str, reader: &'a mut T, store: &'a mut Box<dyn Store>) -> Self {
        Self {
            base_path: base_path.to_string(),
            hasher: Mutex::new(Some((ring::digest::Context::new(&ring::digest::SHA256), 0))), // TODO: other hashing algs
            reader: Mutex::new(reader),
            store: Mutex::new(store),
        }
    }

    pub async fn parse(&'a mut self) -> Result<NarResult, NarError> {
        trace!("starting parsing of nar for path {}", self.base_path);
        let version = self.read_string().await?;
        debug!("got nar with version: '{}'", version);
        if version != NAR_VERSION_MAGIC_1 {
            return Err(NarError::NotAArchive {});
        }

        self.inner_parser(self.base_path.to_owned()).await.await?;

        let (hasher, len) = self.hasher.lock().unwrap().take().unwrap();
        let hash = hasher.finish();
        let hash = Hash::from_sha256_vec(hash.as_ref())?;

        Ok(NarResult {
            // TODO: fix
            len,
            hash,
        })
    }

    pub async fn inner_parser(&'a self, path: String) -> LocalFutureObj<'a, Result<(), NarError>> {
        LocalFutureObj::new(Box::new(async move {
            let tag = self.read_string().await?;
            if tag != "(" {
                return Err(NarError::MissingOpenTag {});
            }

            let mut f_type = Type::Unknown;
            let mut state = State::None;

            loop {
                let s = self.read_string().await?;

                if s == ")" {
                    break;
                } else if s == "type" {
                    let t = self.read_string().await?;
                    if f_type != Type::Unknown {
                        return Err(NarError::MultipleTypeFieleds {});
                    }
                    f_type = Type::from(t.as_str());

                    state = match f_type {
                        Type::Unknown => {
                            return Err(NarError::UnknownFileType { file: t });
                        }
                        Type::Regular => State::File(path.to_owned()),
                        Type::Directory => {
                            debug!("creating directory: '{}'", path);
                            let mut store = self.store.lock().unwrap();
                            store.make_directory(&path).await?;
                            //State::Directory(path.to_owned())
                            State::None // state not needed here
                        }
                        Type::Symlink => {
                            let target = self.read_string().await?;
                            if target != "target" {
                                return Err(NarError::InvalidSymlinkMarker { marker: target });
                            }
                            let target = self.read_string().await?;
                            debug!("creating symlink: '{} -> {}'", path, target);
                            let mut store = self.store.lock().unwrap();
                            store.make_symlink(&path, &target).await?;
                            State::None // state not needed here
                        }
                    }
                } else if s == "contents" {
                    match &state {
                        State::File(v) => {
                            let mut store = self.store.lock().unwrap();
                            store
                                .write_file(&v, &self.read_os_string().await?, false)
                                .await?
                        }
                        State::Executable(v) => {
                            let mut store = self.store.lock().unwrap();
                            store
                                .write_file(&v, &self.read_os_string().await?, true)
                                .await?
                        }
                        _ => return Err(NarError::InvalidState { state: state }),
                    }
                } else if s == "executable" {
                    let s = self.read_string().await?;
                    if s != "" {
                        return Err(NarError::InvalidExecutableMarker {});
                    }
                    state = match state {
                        State::File(v) => State::Executable(v),
                        _ => return Err(NarError::InvalidState { state: state }),
                    };
                } else if s == "entry" {
                    let mut name = String::new();
                    let mut prev_name = String::new();

                    let s = self.read_string().await?;
                    if s != "(" {
                        return Err(NarError::MissingOpenTag {});
                    }
                    loop {
                        let s = self.read_string().await?;
                        if s == ")" {
                            break;
                        } else if s == "name" {
                            name = self.read_string().await?;
                            if name.len() == 0
                                || name == "."
                                || name == ".."
                                || name.find('/') != None
                                || name.find('\0') != None
                            {
                                return Err(NarError::InvalidFileName { name });
                            }
                            if name <= prev_name {
                                return Err(NarError::NotSorted {});
                            }
                            prev_name = name.clone();
                            trace!("parsing node with name: '{}'", name);
                        // TODO: macos case hack
                        } else if s == "node" {
                            if name.is_empty() {
                                return Err(NarError::MissingName {});
                            }
                            self.inner_parser(format!("{}/{}", path, name))
                                .await
                                .await?;
                        }
                    }
                    trace!("exit node loop")
                } else {
                    let v = self.read_string().await?;
                    unimplemented!("what am I doing?: {}", v);
                }
            }
            trace!("finished parsing of '{}'", path);
            Ok(())
        }))
    }

    // TODO: make all these to a trait somehow
    async fn read_int(&'a self) -> Result<u64, io::Error> {
        let mut buf: [u8; 8] = [0; 8];

        let mut reader = self.reader.lock().unwrap();
        reader.read_exact(&mut buf).await?;
        self.hash(&buf);

        // update_hasher

        Ok(LittleEndian::read_u64(&buf))
    }

    async fn read_os_string(&'a self) -> Result<Vec<u8>, NarError> {
        let mut len = self.read_int().await?; // Borrow checker fails here, so will inline this function
                                              /*let mut len = {
                                                  let mut buf: [u8; 8] = [0; 8];
                                                  self.reader.read_exact(&mut buf).await?;
                                                  LittleEndian::read_u64(&buf)
                                              };*/

        let mut buf: [u8; 1024] = [0; 1024];
        let mut value = Vec::new();
        let mut reader = self.reader.lock().unwrap();

        while len > 1024 {
            reader.read_exact(&mut buf).await?;
            value.extend_from_slice(&buf);
            len = len - 1024;
        }

        reader.read_exact(&mut buf[..len as usize]).await?;
        value.extend_from_slice(&buf[..len as usize]);

        drop(reader);

        self.hash(&value);

        // update_hasher
        self.read_padding(len).await?;

        Ok(value)
    }

    async fn read_string(&'a self) -> Result<String, NarError> {
        trace!("read string");
        Ok(String::from_utf8_lossy(&self.read_os_string().await?).to_string())
    }

    /*async fn read_strings(&'a self) -> Result<Vec<String>, NarError> {
        trace!("read strings");
        let len = self.read_int().await?; // borrow checker fails

        let mut vec = Vec::with_capacity(len as usize);
        for v in 0..len {
            vec.push(self.read_string().await?);
        }

        Ok(vec)
    }*/

    async fn read_padding(&'a self, len: u64) -> Result<(), NarError> {
        trace!("read padding");
        if len % 8 != 0 {
            let mut buf: [u8; 8] = [0; 8];
            let len = 8 - (len % 8) as usize;
            trace!("read {} padding", len);

            let mut reader = self.reader.lock().unwrap();
            reader.read_exact(&mut buf[..len]).await?;
            self.hash(&buf[..len]);
            // TODO: check for non 0
        }
        trace!("end of read padding");
        Ok(())
    }

    fn hash(&'a self, data: &[u8]) {
        let mut hasher = self.hasher.lock().unwrap();
        if let Some((hasher, len)) = &mut *hasher {
            hasher.update(data);
            *len += data.len() as u64;
        }
        //if let Some((&mut hasher, &mut len)) = *self.hasher.lock().unwrap() {

        //}
    }
}

#[derive(Debug, PartialEq)]
pub enum Type {
    Unknown,
    Regular,
    Directory,
    Symlink,
}

impl std::convert::From<&str> for Type {
    fn from(v: &str) -> Self {
        match v {
            "regular" => Type::Regular,
            "directory" => Type::Directory,
            "symlink" => Type::Symlink,
            _ => Type::Unknown,
        }
    }
}

#[derive(Debug)]
pub enum State {
    // TODO: only store references for less memory footprint?
    None,
    File(String),
    Executable(String),
    //Directory(String),
    //Symlink(String),
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self) // TODO: write match cases
    }
}

#[cfg(test)]
mod test {
    use super::{NarError, NarParser};
    use crate::store::mock_store::MockStore;
    use crate::store::{Store, StoreError};
    use env_logger;
    use log::info;
    use tokio::io::AsyncRead;

    #[tokio::test]
    async fn read_simple_file() {
        match env_logger::try_init() {
            // this may file because of previos test
            _ => (),
        }
        let store = MockStore::new();
        let mut box_store = Box::new(store) as Box<dyn Store>;

        // this is skipped in rustfmt to see packet boundings
        #[rustfmt::skip]
        let mut reader: &[u8] = &[
            13, 0, 0, 0, 0, 0, 0, 0, 110, 105, 120,  45,  97, 114,  99, 104, 105, 118, 101, 45, 49, 0, 0, 0,
             1, 0, 0, 0, 0, 0, 0, 0,  40,   0,   0,   0,   0,   0,   0,   0,
             4, 0, 0, 0, 0, 0, 0, 0, 116, 121, 112, 101,   0,   0,   0,   0,
             7, 0, 0, 0, 0, 0, 0, 0, 114, 101, 103, 117, 108,  97, 114,   0,
             8, 0, 0, 0, 0, 0, 0, 0,  99, 111, 110, 116, 101, 110, 116, 115,
             5, 0, 0, 0, 0, 0, 0, 0, 104, 101, 108, 108, 111,   0,   0,   0,
             1, 0, 0, 0, 0, 0, 0, 0,  41,   0,   0,   0,   0,   0,   0,   0,
        ];

        let mut parser = NarParser::new("/mock/string", &mut reader, &mut box_store);

        println!("running parser");
        let ret = parser.parse().await.unwrap();

        let b: &MockStore = box_store
            .as_any()
            .take()
            .unwrap()
            .downcast_ref::<MockStore>()
            .unwrap();

        assert!(b.file_exists("/mock/string"));
        assert!(b.file_as_string("/mock/string").eq("hello"));
        assert!(!b.is_file_executable("/mock/string"));
    }

    #[tokio::test]
    async fn read_dir() {
        match env_logger::try_init() {
            // this may file because of previos test
            _ => (),
        }
        let store = MockStore::new();
        let mut box_store = Box::new(store) as Box<dyn Store>;

        let mut reader: &[u8] = &[
            // created via `nix dump-path`
            0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x69, 0x78, 0x2d, 0x61, 0x72,
            0x63, 0x68, 0x69, 0x76, 0x65, 0x2d, 0x31, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0x79, 0x70, 0x65, 0x00, 0x00, 0x00, 0x00,
            0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x69, 0x72, 0x65, 0x63, 0x74,
            0x6f, 0x72, 0x79, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x65, 0x6e, 0x74, 0x72, 0x79, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x61, 0x6d, 0x65, 0x00, 0x00,
            0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x65, 0x78, 0x65, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x6f,
            0x64, 0x65, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x74, 0x79, 0x70, 0x65, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x72, 0x65, 0x67, 0x75, 0x6c, 0x61, 0x72, 0x00, 0x0a, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x65, 0x78, 0x65, 0x63, 0x75, 0x74, 0x61, 0x62,
            0x6c, 0x65, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x6f, 0x6e, 0x74,
            0x65, 0x6e, 0x74, 0x73, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x65, 0x78,
            0x65, 0x63, 0x75, 0x74, 0x65, 0x0a, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x65, 0x6e, 0x74, 0x72, 0x79, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x61, 0x6d, 0x65, 0x00, 0x00,
            0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x65, 0x78, 0x65, 0x5f,
            0x73, 0x79, 0x6d, 0x6c, 0x69, 0x6e, 0x6b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x6f, 0x64, 0x65, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0x79, 0x70, 0x65,
            0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x73, 0x79,
            0x6d, 0x6c, 0x69, 0x6e, 0x6b, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x74, 0x61, 0x72, 0x67, 0x65, 0x74, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x65, 0x78, 0x65, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x65, 0x6e, 0x74, 0x72, 0x79, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x61,
            0x6d, 0x65, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x66, 0x69, 0x6c, 0x65, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x6e, 0x6f, 0x64, 0x65, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0x79, 0x70, 0x65, 0x00, 0x00, 0x00, 0x00,
            0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x72, 0x65, 0x67, 0x75, 0x6c, 0x61,
            0x72, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x6f, 0x6e, 0x74,
            0x65, 0x6e, 0x74, 0x73, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x68, 0x65,
            0x6c, 0x6c, 0x6f, 0x0a, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut parser = NarParser::new("/mock/dir", &mut reader, &mut box_store);

        println!("running parser");
        let ret = parser.parse().await.unwrap();

        let b: &MockStore = box_store
            .as_any()
            .take()
            .unwrap()
            .downcast_ref::<MockStore>()
            .unwrap();

        assert!(b.dir_exists("/mock/dir"));
        assert!(b.file_exists("/mock/dir/file"));
        assert_eq!(b.file_as_string("/mock/dir/file"), "hello\n");
        assert!(!b.is_file_executable("/mock/dir/file"));

        assert!(b.file_exists("/mock/dir/exe"));
        assert_eq!(b.file_as_string("/mock/dir/exe"), "execute\n");
        assert!(b.is_file_executable("/mock/dir/exe"));

        assert!(b.link_exists("/mock/dir/exe_symlink"));
        assert_eq!(b.symlinks_points_at("/mock/dir/exe_symlink"), "exe");

        assert_eq!(ret.len, 712);
        assert_eq!(
            ret.hash.to_base32().unwrap(),
            "KTDZMKFP5AQGNPGXF5NVF4I3L23V3IQLACSPGYMC44LSA23NNFTQ===="
        );
    }
}
