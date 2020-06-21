use std::sync::{Arc, RwLock};

use byteorder::{ByteOrder, LittleEndian};

use log::{debug, error, trace};

use tokio::io::AsyncReadExt;
use tokio::net::unix::{ReadHalf, WriteHalf};
use tokio::net::UnixStream;

use crate::error::StoreError;
type EmptyResult = Result<(), StoreError>;

pub const WORKER_MAGIC_1: u32 = 0x6e697863;
pub const WORKER_MAGIC_2: u32 = 0x6478696f;
pub const PROTOCOL_VERSION: u16 = 0x115;

pub mod logger;

#[derive(Debug)]
struct ClientSettings {
    keep_failed: bool,
    keep_going: bool,
    try_fallback: bool,
    verbosity: crate::store::protocol::Verbosity,
    max_build_jobs: u32,
    max_silent_time: u32,
    build_cores: u32,
    use_substitutes: bool,
    overrides: std::collections::HashMap<String, Data>,
}

impl ClientSettings {
    pub fn new() -> Self {
        Self {
            keep_failed: false,
            keep_going: false,
            try_fallback: false,
            verbosity: crate::store::protocol::Verbosity::LVLError,
            max_build_jobs: 0,
            max_silent_time: 0,
            build_cores: 0,
            use_substitutes: false,
            overrides: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug)]
enum Data {
    String(String),
    USize(usize),
}

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

        loop {
            // daemon loop
            let command = self.read_int().await?;

            // let command = Command::from(command);
            let command = crate::store::protocol::WorkerOp::from(command as u32);
            println!("command: {:?}", command);
            self.perform_op(command).await?;
        }

        //Ok(())
    }

    async fn perform_op(&mut self, command: crate::store::protocol::WorkerOp) -> EmptyResult {
        use crate::store::protocol::WorkerOp;

        match command {
            WorkerOp::WopInvalidRequest => Ok(()),
            WorkerOp::WopSetOptions => self.set_options().await,
            _ => {
                error!("not yet implemented");
                Ok(())
            }
        }
    }

    async fn set_options(&mut self) -> EmptyResult {
        let mut settings = ClientSettings::new();

        settings.keep_failed = self.read_int().await? != 0;
        settings.keep_going = self.read_int().await? != 0;
        settings.try_fallback = self.read_int().await? != 0;
        settings.verbosity = crate::store::protocol::Verbosity::from(self.read_int().await? as u32);
        settings.max_build_jobs = self.read_int().await? as u32;
        settings.max_silent_time = self.read_int().await? as u32;
        self.read_int().await?; // obsolete: useBuildHook
        self.read_int().await?; // FIXME: verbose build
        self.read_int().await?; // obsolete: logType
        self.read_int().await?; // obsolete: printBuildTrace
        settings.build_cores = self.read_int().await? as u32;
        settings.use_substitutes = self.read_int().await? != 0;

        // TODO: check for client version >= 12
        let n = self.read_int().await?;
        trace!("{} extra options", n);
        for i in 0..n {
            let name = self.read_string().await?;
            let value = self.read_string().await?;
            settings.overrides.insert(name, Data::String(value));
        }

        println!("settings: {:?}", settings);

        Ok(())
    }

    async fn read_int(&self) -> std::io::Result<u64> {
        let mut reader = self.reader.write().unwrap();
        let mut buf: [u8; 8] = [0; 8];

        reader.read_exact(&mut buf).await?;

        Ok(LittleEndian::read_u64(&buf))
    }

    async fn read_string(&self) -> Result<String, StoreError> {
        let len = self.read_int().await?; // FIXME: orignal uses size_t??
        let mut buf: [u8; 100] = [0; 100];

        trace!("reading string with len {}", len);

        let mut reader = self.reader.write().unwrap();
        reader.read_exact(&mut buf[0..len as usize]).await?;
        drop(reader); // release reader

        let value = String::from_utf8_lossy(&buf[0..len as usize]).to_string();

        // FIXME: read padding
        self.read_padding(len).await?;

        trace!("read string {}", value);

        Ok(value)
    }

    async fn read_padding(&self, len: u64) -> EmptyResult {
        if len % 8 != 0 {
            let mut buf: [u8; 8] = [0; 8];
            let len = 8 - (len % 8) as usize;

            let mut reader = self.reader.write().unwrap();
            reader.read_exact(&mut buf[0..len]).await?;
            // TODO: check for non 0 values
        }
        Ok(())
    }
}
