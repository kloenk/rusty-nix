use std::sync::{Arc, RwLock};

use tokio::io::AsyncReadExt;
use tokio::net::unix::{ReadHalf, WriteHalf};
use tokio::net::UnixStream;

pub const WORKER_MAGIC_1: u32 = 0x6e697863;
pub const WORKER_MAGIC_2: u32 = 0x6478696f;
pub const PROTOCOL_VERSION: u16 = 0x115;

pub mod logger;

pub struct Connection<'a> {
    pub trusted: bool,

    logger: logger::TunnelLogger<'a>,

    writer: Arc<RwLock<WriteHalf<'a>>>,
    reader: Arc<RwLock<ReadHalf<'a>>>,

    uid: u32,
    u_name: String,

    store: Box<dyn crate::Store>,
}

impl<'a> Connection<'a> {
    pub fn new(
        trusted: bool,
        client_version: u16,
        stream: &'a mut UnixStream,
        store: Box<dyn crate::Store>,
        uid: u32,
        u_name: String,
    ) -> Self {
        let (reader, writer) = stream.split();
        let reader = Arc::new(RwLock::new(reader));
        let writer = Arc::new(RwLock::new(writer));

        let logger = logger::TunnelLogger::new(client_version, writer.clone());
        Self {
            trusted,
            logger,
            reader,
            writer,
            store,
            uid,
            u_name,
        }
    }

    pub async fn run(mut self) -> Result<(), crate::error::StoreError> {
        self.logger.start_work().await?;

        self.store
            .create_user(self.u_name.clone(), self.uid)
            .await?;
        self.logger.stop_work(logger::WorkDone).await?;

        let mut buffer: [u8; 32] = [0; 32];
        let mut reader = self.reader.write().unwrap();
        reader.read(&mut buffer);

        println!("read: {:?}", buffer);

        Ok(())
    }
}
