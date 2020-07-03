use byteorder::{ByteOrder, LittleEndian};
use tokio::io::{AsyncRead, AsyncReadExt};

use futures::future::LocalFutureObj;
use std::boxed::Box;

use crate::error::NarError;
use crate::Store;

use std::io;
use std::sync::Mutex;

pub const NAR_VERSION_MAGIC_1: &'static str = "nix-archive-1";

/// Returned as succesfully parsed nar archive
pub struct NarResult {
    pub path: String,
    //pub hash: Hash,
    pub len: u64,
}

pub struct NarParser<'a, T: ?Sized + AsyncRead + Unpin> {
    reader: Mutex<&'a mut T>,

    store: Mutex<&'a mut Box<dyn Store>>,

    pub base_path: String,
}

impl<'a, T: ?Sized + AsyncRead + Unpin> NarParser<'a, T> {
    pub fn new(base_path: &str, reader: &'a mut T, store: &'a mut Box<dyn Store>) -> Self {
        Self {
            base_path: base_path.to_string(),
            reader: Mutex::new(reader),
            store: Mutex::new(store),
        }
    }

    pub async fn parse(&'a mut self) -> Result<NarResult, NarError> {
        // TODO: implement Error
        let version = self.read_string().await?;
        if version != NAR_VERSION_MAGIC_1 {
            return Err(NarError::NotAArchive {});
        }

        //self.inner_parser().await?;
        unimplemented!()
    }

    pub async fn inner_parser(&'a self, path: String) -> LocalFutureObj<'a, Result<(), NarError>> {
        LocalFutureObj::new(Box::new(async move {
            let tag = self.read_string().await?;
            if tag != "(" {
                return Err(NarError::MissingOpenTag {});
            }

            let mut f_type = Type::Unknown;
            let mut state = State::None;
            let mut store = self.store.lock().unwrap();

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
                        _ => unimplemented!()
                    }
                } else if s == "contents" {
                    match state {
                        State::File(v) => store.write_regular_file(v, self.read_os_string().await?).await?;
                        _ => unimplemented!(),
                    }
                }
            }
            Ok(())
        }))
    }

    // TODO: make all these to a trait somehow
    async fn read_int(&'a self) -> Result<u64, io::Error> {
        let mut buf: [u8; 8] = [0; 8];

        let mut reader = self.reader.lock().unwrap();
        reader.read_exact(&mut buf).await?;

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

        // update_hasher
        self.read_padding(len).await?;

        Ok(value)
    }

    async fn read_string(&'a self) -> Result<String, NarError> {
        Ok(String::from_utf8_lossy(&self.read_os_string().await?).to_string())
    }

    async fn read_strings(&'a self) -> Result<Vec<String>, NarError> {
        // FIXME: currently broken
        let len = self.read_int().await?; // borrow checker fails

        let mut vec = Vec::with_capacity(len as usize);
        for v in 0..len {
            vec.push(self.read_string().await?);
        }

        Ok(vec)
    }

    async fn read_padding(&'a self, len: u64) -> Result<(), NarError> {
        if len % 8 != 0 {
            let mut buf: [u8; 8] = [0; 8];
            let len = 8 - (len % 8) as usize;

            let mut reader = self.reader.lock().unwrap();
            reader.read_exact(&mut buf[..len]).await?;
            // TODO: check for non 0
        }
        Ok(())
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

pub enum State { // TODO: only store references for less memory footprint?
    None,
    File(String),
    Executable(String),
    Directory(String),
    Symlink(String),
}
