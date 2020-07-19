use std::sync::{Arc, Mutex};

use tokio::io::AsyncReadExt;
use tokio::net::unix::{ReadHalf, WriteHalf};

use byteorder::{ByteOrder, LittleEndian};

/// These are exported, because there are needed for async traits
use futures::future::LocalFutureObj;
/// These are exported, because there are needed for async traits
use std::boxed::Box;

// TODO: add logger trait for logging

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

type EmtyResult = Result<(), std::io::Error>;

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

    fn write_bool<'a>(&'a self, v: bool) -> LocalFutureObj<'a, EmtyResult> {
        self.write_u64(v as u64)
    }

    fn write_string<'a>(&'a self, str: &'a str) -> LocalFutureObj<'a, EmtyResult> {
        LocalFutureObj::new(Box::new(async move {
            self.write_u64(str.len() as u64).await?;

            let v = self.write(str.as_bytes()).await?;
            ieieo(v, str.len())?;

            self.write_padding(str.len()).await?;

            Ok(())
        }))
    }

    fn write_padding<'a>(&'a self, len: usize) -> LocalFutureObj<'a, EmtyResult> {
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

    fn write_strings<'a>(&'a self, v: &'a Vec<String>) -> LocalFutureObj<'a, EmtyResult> {
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
        return Err(std::io::Error::from_raw_os_error(libc::EIO));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Connection<'a> {
    pub reader: Arc<Mutex<ReadHalf<'a>>>,
    pub writer: Arc<Mutex<WriteHalf<'a>>>,
}

impl<'b> Connection<'b> {
    pub fn new(reader: ReadHalf<'b>, writer: WriteHalf<'b>) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    pub fn new_arc(reader: Arc<Mutex<ReadHalf<'b>>>, writer: Arc<Mutex<WriteHalf<'b>>>) -> Self {
        Self { reader, writer }
    }
}

impl<'b> AsyncRead for Connection<'b> {
    fn read_exact<'a>(
        &'a self,
        buf: &'a mut [u8],
        len: usize,
    ) -> LocalFutureObj<'a, Result<usize, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            if buf.len() < len {
                return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
            }

            // TODO: add optional hasher

            let mut reader = self.reader.lock().unwrap();
            Ok(reader.read_exact(&mut buf[0..len]).await?)
        }))
    }
}

impl<'b> AsyncWrite for Connection<'b> {
    fn write<'a>(&'a self, buf: &'a [u8]) -> LocalFutureObj<'a, Result<usize, std::io::Error>> {
        LocalFutureObj::new(Box::new(async move {
            use tokio::io::AsyncWriteExt;
            let mut writer = self.writer.lock().unwrap(); // TODO: return EBUSSY
            Ok(writer.write(buf).await?)
        }))
    }
}

#[cfg(test)]
mod test {
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
}
