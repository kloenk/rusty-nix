use std::sync::{Arc, Mutex};

use tokio::io::AsyncReadExt;
use tokio::net::unix::{ReadHalf, WriteHalf};

use byteorder::{ByteOrder, LittleEndian};

/// These are exported, because there are needed for async traits
use futures::future::LocalFutureObj;
/// These are exported, because there are needed for async traits
use std::boxed::Box;

mod logger;
pub use logger::{Logger, WorkFinish, STDERR};

/// Shortcut for `WorkFinish::Done`
pub const WORKDONE: WorkFinish = WorkFinish::Done;

pub trait AsyncRead {
    fn read_exact<'a>(
        &'a self,
        buf: &'a mut [u8],
        len: usize,
    ) -> LocalFutureObj<'a, Result<usize, std::io::Error>>;

    fn read_u64<'a>(&'a self) -> LocalFutureObj<'a, Result<u64, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            let mut buf: [u8; 8] = [0; 8];

            self.read_exact(&mut buf, 8).await?;

            Ok(LittleEndian::read_u64(&buf))
        }))
    }

    fn read_bool<'a>(&'a self) -> LocalFutureObj<'a, Result<bool, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move { Ok(self.read_u64().await? == 0) }))
    }

    fn read_os_string<'a>(&'a self) -> LocalFutureObj<'a, Result<Vec<u8>, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            let len = self.read_u64().await? as usize;

            let mut buf = Vec::with_capacity(len);
            buf.resize(len, 0);

            let read = self.read_exact(buf.as_mut_slice(), len).await?;
            self.read_padding(len).await?;

            assert_eq!(read, len); // TODO: better error
            Ok(buf)
        }))
    }

    fn read_padding<'a>(&'a self, len: usize) -> LocalFutureObj<'a, Result<(), std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            if len % 8 != 0 {
                let len: usize = 8 - (len % 8) as usize;

                let mut buf = Vec::with_capacity(len);
                buf.resize(len, 0);

                self.read_exact(&mut buf, len).await?;

                for v in buf {
                    if v != 0 {
                        log::warn!("padding is non zero");
                        return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
                    }
                }
            }

            Ok(())
        }))
    }

    fn read_string<'a>(&'a self) -> LocalFutureObj<'a, Result<String, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            Ok(String::from_utf8_lossy(&self.read_os_string().await?).to_string())
        }))
    }

    fn read_strings<'a>(&'a self) -> LocalFutureObj<'a, Result<Vec<String>, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            let len = self.read_u64().await?;

            let mut vec = Vec::with_capacity(len as usize);
            for _v in 0..len {
                vec.push(self.read_string().await?);
            }

            Ok(vec)
        }))
    }
}

type EmptyResult = Result<(), std::io::Error>;

// TODO: flush?
pub trait AsyncWrite {
    fn write<'a>(&'a self, buf: &'a [u8]) -> LocalFutureObj<'a, Result<usize, std::io::Error>>;

    fn write_u64<'a>(&'a self, v: u64) -> LocalFutureObj<'a, Result<(), std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            let mut buf: [u8; 8] = [0; 8];
            LittleEndian::write_u64(&mut buf, v);

            let v = self.write(&buf).await?;
            ieieo(v, 8)?;
            Ok(())
        }))
    }

    fn write_bool<'a>(&'a self, v: bool) -> LocalFutureObj<'a, EmptyResult> {
        self.write_u64(v as u64)
    }

    fn write_string<'a>(&'a self, str: &'a str) -> LocalFutureObj<'a, EmptyResult> {
        LocalFutureObj::new(Box::new(async move {
            self.write_u64(str.len() as u64).await?;

            let v = self.write(str.as_bytes()).await?;
            ieieo(v, str.len())?;

            self.write_padding(str.len()).await?;

            Ok(())
        }))
    }

    fn write_padding<'a>(&'a self, len: usize) -> LocalFutureObj<'a, EmptyResult> {
        LocalFutureObj::new(Box::new(async move {
            if len % 8 != 0 {
                let len = 8 - (len % 8);
                let mut buf = Vec::with_capacity(len);
                buf.resize(len, 0);

                let v = self.write(buf.as_slice()).await?;
                ieieo(v, len)?;
            }

            Ok(())
        }))
    }

    fn write_strings<'a>(&'a self, v: &'a Vec<String>) -> LocalFutureObj<'a, EmptyResult> {
        LocalFutureObj::new(Box::new(async move {
            self.write_u64(v.len() as u64).await?;

            for v in v {
                self.write_string(v).await?;
            }

            Ok(())
        }))
    }
}

#[inline(always)]
// TODO: make as macro
fn ieieo(act: usize, expt: usize) -> Result<(), std::io::Error> {
    if act != expt {
        log::warn!("ieieo! act: {}, expt: {}", act, expt);
        return Err(std::io::Error::from_raw_os_error(libc::EIO));
    }
    Ok(())
}

pub struct HashResult {
    pub hash: crate::store::Hash,
    pub size: usize,
}

#[derive(Clone)] // TODO: add Debug (not supported by Context)
pub struct Connection {
    pub stream: Arc<Mutex<tokio::net::UnixStream>>,
    pub hasher: Arc<Mutex<Option<(usize, ring::digest::Context)>>>,

    // tunnelsource flag
    pub tunnelsource: Arc<std::sync::atomic::AtomicBool>,

    // logger types
    pub can_send: Arc<std::sync::atomic::AtomicBool>,
    pub pending_msgs: Arc<Mutex<Vec<String>>>,
}

impl Connection {
    pub fn new(stream: tokio::net::UnixStream) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
            hasher: Arc::new(Mutex::new(None)),

            tunnelsource: Arc::new(std::sync::atomic::AtomicBool::new(false)),

            can_send: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pending_msgs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn new_arc(stream: Arc<Mutex<tokio::net::UnixStream>>) -> Self {
        Self {
            stream,
            hasher: Arc::new(Mutex::new(None)),

            tunnelsource: Arc::new(std::sync::atomic::AtomicBool::new(false)),

            can_send: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pending_msgs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_hasher(&self) -> Result<(), std::io::Error> {
        let mut hasher = self.hasher.lock().unwrap();
        if hasher.is_some() {
            return Err(std::io::Error::from_raw_os_error(libc::EFAULT));
        }

        *hasher = Some((0, ring::digest::Context::new(&ring::digest::SHA256)));

        Ok(())
    }

    pub fn get_hasher(
        &self,
    ) -> Result<Arc<Mutex<Option<(usize, ring::digest::Context)>>>, std::io::Error> {
        Ok(self.hasher.clone())
    }

    pub fn pop_hasher(&self) -> Result<HashResult, crate::StoreError> {
        let hasher = self.hasher.lock().unwrap().take();

        if hasher.is_none() {
            Err(std::io::Error::from_raw_os_error(libc::EFAULT))?;
        }
        let (size, hasher) = hasher.unwrap();

        Ok(HashResult {
            hash: crate::store::Hash::from_sha256_vec(hasher.finish().as_ref())?,
            size,
        })
    }

    pub fn update_hash(&self, size: usize, buf: &[u8]) {
        let mut hasher = self.hasher.lock().unwrap();
        if let Some(v) = &mut *hasher {
            v.0 += size;
            v.1.update(buf);
        }
    }

    pub fn set_tunnel(&self, tunnel: bool) -> bool {
        self.tunnelsource.swap(tunnel, Ordering::Relaxed)
    }

    pub fn get_tunnel(&self) -> bool {
        self.tunnelsource.load(Ordering::Relaxed)
    }
}

impl AsyncRead for Connection {
    fn read_exact<'a>(
        &'a self,
        buf: &'a mut [u8],
        len: usize,
    ) -> LocalFutureObj<'a, Result<usize, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            if buf.len() < len {
                return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
            }

            let size = if self.get_tunnel() {
                self.write_u64(STDERR::READ as u64).await?;
                self.write_u64(len as u64).await?;
                log::trace!("requesting {} bytes from source", len);

                let mut buf_int: [u8; 8] = [0; 8];
                let mut reader = self.stream.lock().unwrap();
                reader.read_exact(&mut buf_int).await?;
                let len_send = LittleEndian::read_u64(&buf_int);
                ieieo(len_send as usize, len)?;

                let size = reader.read_exact(&mut buf[0..len]).await?;

                if len % 8 != 0 {
                    let len: usize = 8 - (len % 8) as usize;

                    let mut buf = Vec::with_capacity(len);
                    buf.resize(len, 0);

                    reader.read_exact(&mut buf[0..len]).await?;
                    //self.read_exact(&mut buf, len).await?;

                    for v in buf {
                        if v != 0 {
                            log::warn!("padding is non zero");
                            return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
                        }
                    }
                }
                size
            } else {
                let mut reader = self.stream.lock().unwrap();
                let size = reader.read_exact(&mut buf[0..len]).await?;
                size
            };

            /*let mut reader = if self.get_tunnel() {
                self.write_u64(STDERR::READ as u64).await?;
                self.write_u64(len as u64).await?;
                log::trace!("requesting {} bytes from source", len);

                let mut buf: [u8; 8] = [0; 8];
                let mut reader = self.stream.lock().unwrap();
                reader.read_exact(&mut buf).await?;
                let len_send = LittleEndian::read_u64(&buf);
                ieieo(len_send as usize, len)?;
                reader
            } else {
                self.stream.lock().unwrap()
            };

            //let mut reader = self.reader.lock().unwrap();
            let size = reader.read_exact(&mut buf[0..len]).await?;*/
            self.update_hash(size, &buf);
            Ok(size)
        }))
    }
}

impl AsyncWrite for Connection {
    fn write<'a>(&'a self, buf: &'a [u8]) -> LocalFutureObj<'a, Result<usize, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            use tokio::io::AsyncWriteExt;
            let mut writer = self.stream.lock().unwrap(); // TODO: return EBUSSY
            Ok(writer.write(buf).await?)
        }))
    }
}

use std::sync::atomic::Ordering;
impl Logger for Connection {
    fn can_send(&self) -> bool {
        self.can_send.load(Ordering::Relaxed)
    }

    fn set_can_send(&self, can: bool) {
        self.can_send.store(can, Ordering::Relaxed)
    }

    fn enqueu(&self, msg: String) {
        let mut queu = self.pending_msgs.lock().unwrap();
        queu.push(msg);
    }

    fn dequeu(&self) -> Vec<String> {
        let mut queu = self.pending_msgs.lock().unwrap();
        let ret = queu.clone();
        queu.clear();

        ret
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::{Arc, AsyncRead, AsyncReadExt, AsyncWrite, Box, LocalFutureObj, Mutex};
    use std::io::Cursor;
    pub struct Connection {
        pub reader: Arc<Mutex<Cursor<Vec<u8>>>>,
        pub writer: Arc<Mutex<Cursor<Vec<u8>>>>,
        pub tunnel: bool,
    }

    impl Connection {
        pub fn new(vec: Vec<u8>, tunnel: bool) -> Self {
            Self {
                reader: Arc::new(Mutex::new(Cursor::new(vec))),
                writer: Arc::new(Mutex::new(Cursor::new(Vec::new()))),
                tunnel,
            }
        }

        pub fn new_empty(tunnel: bool) -> Self {
            Self::new(vec![], tunnel)
        }
    }

    impl AsyncRead for Connection {
        fn read_exact<'a>(
            &'a self,
            buf: &'a mut [u8],
            len: usize,
        ) -> LocalFutureObj<'a, Result<usize, std::io::Error>> {
            LocalFutureObj::new(Box::new(async move {
                if buf.len() < len {
                    println!("slice to small: '{}'", buf.len());
                    return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
                }

                let mut reader = self.reader.lock().unwrap();
                let read = (*reader).read_exact(&mut buf[0..len]).await?;
                println!("trace: read: '{:?}'", buf);
                Ok(read)
            }))
        }
    }

    impl AsyncWrite for Connection {
        fn write<'a>(&'a self, buf: &'a [u8]) -> LocalFutureObj<'a, Result<usize, std::io::Error>> {
            LocalFutureObj::new(Box::new(async move {
                use tokio::io::AsyncWriteExt;
                let mut writer = self.writer.lock().unwrap();

                Ok(writer.write(buf).await?)
            }))
        }
    }

    #[tokio::test]
    async fn read_u64() {
        let con = Connection::new(vec![2, 0, 0, 0, 0, 0, 0, 0], false);
        let data = con.read_u64().await.unwrap();

        let pos = con.reader.lock().unwrap().position();

        assert_eq!(data, 2);
        assert_eq!(pos, 8);
        // TODO: check tunnel data
    }

    #[tokio::test]
    async fn read_os_string() {
        let con = Connection::new(
            vec![
                15, 0, 0, 0, 0, 0, 0, 0, 5, 5, 0, 0, 0, 0, 0, 0, 6, 6, 0, 0, 0, 0, 0, 0,
            ],
            false,
        );

        let data = con.read_os_string().await.unwrap();

        let pos = con.reader.lock().unwrap().position();

        assert_eq!(data, vec![5, 5, 0, 0, 0, 0, 0, 0, 6, 6, 0, 0, 0, 0, 0]);
        assert_eq!(pos, 24);
    }

    #[tokio::test]
    async fn read_string() {
        let con = Connection::new(
            vec![2, 0, 0, 0, 0, 0, 0, 0, 63, 61, 0, 0, 0, 0, 0, 0],
            false,
        );

        let str = con.read_string().await.unwrap();

        let pos = con.reader.lock().unwrap().position();

        assert_eq!(str, "?=");
        assert_eq!(pos, 16);
    }

    #[tokio::test]
    async fn read_strings() {
        // skip to see boundaries
        #[rustfmt::skip]
        let con = Connection::new(vec![
             2,   0,   0,   0, 0, 0, 0, 0,
             2,   0,   0,   0, 0, 0, 0, 0,
            63,  61,   0,   0, 0, 0, 0, 0,
             4,   0,   0,   0, 0, 0, 0, 0,
            70, 105, 110, 110, 0, 0, 0, 0,

        ], false);

        let strs = con.read_strings().await.unwrap();

        let pos = con.reader.lock().unwrap().position();

        assert_eq!(strs, vec!["?=", "Finn"]);
        assert_eq!(pos, 40);
    }

    #[tokio::test]
    async fn write_u64() {
        let con = Connection::new_empty(false);

        con.write_u64(2).await.unwrap();

        let writer = con.writer.lock().unwrap();
        let vec: Vec<u8> = vec![2, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(writer.get_ref(), &vec);
    }

    // TODO: logger tests
}
