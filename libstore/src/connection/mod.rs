use std::sync::{Arc, RwLock};

use byteorder::{ByteOrder, LittleEndian};

use log::*;

#[allow(unused_imports)]
use futures::future::LocalFutureObj;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::unix::{ReadHalf, WriteHalf};
use tokio::net::UnixStream;

use crate::error::StoreError;
type EmptyResult = Result<(), StoreError>;

pub const WORKER_MAGIC_1: u32 = 0x6e697863;
pub const WORKER_MAGIC_2: u32 = 0x6478696f;
pub const PROTOCOL_VERSION: u16 = 0x115;

#[allow(unused_imports)]
use crate::unimplemented;

pub mod logger;
//pub mod archive;

pub const NARVERSIONMAGIC_1: &str = "nix-archive-1";

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
    overrides: std::collections::HashMap<String, Data>, // TODO:: use libstore::store::Param
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
#[allow(dead_code)]
enum Data {
    String(String),
    USize(usize),
}

pub struct Connection<'a> {
    pub trusted: bool,

    logger: logger::TunnelLogger<'a>,

    writer: Arc<RwLock<WriteHalf<'a>>>,
    reader: Arc<RwLock<ReadHalf<'a>>>,

    hasher: RwLock<Option<(ring::digest::Context, usize)>>,

    uid: u32,
    u_name: String,

    store: Box<dyn crate::store::WriteStore>,
}

impl<'a> Connection<'a> {
    pub fn new(
        trusted: bool,
        client_version: u16,
        stream: &'a mut UnixStream,
        store: Box<dyn crate::store::WriteStore>,
        uid: u32,
        u_name: String,
    ) -> Self {
        let (reader, writer) = stream.split();
        let reader = Arc::new(RwLock::new(reader));
        let writer = Arc::new(RwLock::new(writer));

        let hasher = RwLock::new(None);

        let logger = logger::TunnelLogger::new(client_version, writer.clone());
        Self {
            trusted,
            logger,
            reader,
            writer,
            hasher,
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
        self.logger.stop_work(logger::WORKDONE).await?;

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
            WorkerOp::WopQueryPathInfo => self.query_path_info().await,
            WorkerOp::WopIsValidPath => self.is_valid_path().await,
            WorkerOp::WopAddTempRoot => self.add_temp_root().await,
            WorkerOp::WopAddIndirectRoot => self.add_indirect_root().await,
            WorkerOp::WopSyncWithGC => self.sync_with_gc().await,
            WorkerOp::WopAddToStoreNar => self.add_to_store_nar().await,
            WorkerOp::WopAddToStore => self.add_to_store().await,
            WorkerOp::WopEnsurePath => self.ensure_path().await,
            WorkerOp::WopAddTextToStore => self.add_text_to_store().await,
            WorkerOp::WopBuildPaths => self.build_paths().await,
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
        for _i in 0..n {
            let name = self.read_string().await?;
            let value = self.read_string().await?;
            settings.overrides.insert(name, Data::String(value));
            warn!("set options not yet fully implemented");
        }

        self.logger.start_work().await?;
        println!("settings: {:?}", settings);
        // FIXME: apply settings (when not recursive)
        self.logger.stop_work(logger::WORKDONE).await?;

        Ok(())
    }

    async fn query_path_info(&mut self) -> EmptyResult {
        let path = self.read_string().await?;
        debug!("queriying path info for {}", path);
        self.logger.start_work().await?;
        let info = self.store.query_path_info(&path).await;
        self.logger.stop_work(logger::WORKDONE).await?;

        match info {
            Err(e) => {
                trace!("no path info: {}", e);
                let mut writer = self.writer.write().unwrap();
                let buf: [u8; 8] = [0; 8];
                writer.write(&buf).await?;
                drop(writer);
            }
            Ok(v) => {
                self.write_u64(1).await?;
                if let Some(v) = v.deriver {
                    self.write_string(&self.store.print_store_path(&v)?).await?;
                } else {
                    self.write_string("").await?;
                }
                self.write_string(&v.nar_hash.to_string()).await?; // TODO: change to sha2 crate
                self.write_u64(v.references.len() as u64).await?;
                for v in &v.references {
                    self.write_string(&self.store.print_store_path(v)?).await?;
                }
                self.write_u64(v.registration_time.timestamp() as u64)
                    .await?;
                if let Some(v) = v.nar_size {
                    self.write_u64(v).await?;
                } else {
                    self.write_u64(0).await?;
                }

                self.write_bool(v.ultimate).await?;
                self.write_strings(&v.sigs).await?;
                if let Some(ca) = v.ca {
                    self.write_string(&ca).await?;
                } else {
                    self.write_string("").await?;
                }
            }
        }

        Ok(())
    }

    async fn is_valid_path(&mut self) -> EmptyResult {
        let path = self.read_string().await?;
        let path = std::path::PathBuf::from(&path);

        debug!("checking if {} is a valid path", path.display());

        self.logger.start_work().await?;
        let valid = self.store.is_valid_path(&path).await?;
        self.logger.stop_work(logger::WORKDONE).await?;
        self.write_bool(valid).await?;

        Ok(())
    }

    async fn add_temp_root(&mut self) -> EmptyResult {
        let path = self.read_string().await?;
        let path = std::path::PathBuf::from(&path);

        debug!("adding temp root for {}", path.display());

        self.logger.start_work().await?;
        warn!("implement add temp root"); // TODO: add temp root
                                          //self.store.add_temp_root(&path).await?;
        self.logger.stop_work(logger::WORKDONE).await?;
        self.write_u64(1).await?;

        Ok(())
    }

    async fn add_indirect_root(&mut self) -> EmptyResult {
        let path = self.read_string().await?;
        let path = std::path::PathBuf::from(&path);

        debug!("adding indirect root for {}", path.display());

        self.logger.start_work().await?;
        // TODO: store.add_indirect_root(&path).await?;
        warn!("implement indirect root");
        self.logger.stop_work(logger::WORKDONE).await?;
        self.write_u64(1).await?;

        Ok(())
    }

    async fn sync_with_gc(&mut self) -> EmptyResult {
        debug!("syncing with gc");

        self.logger.start_work().await?;
        // TODO: store.add_indirect_root(&path).await?;
        warn!("implement gc sync");
        self.logger.stop_work(logger::WORKDONE).await?;
        self.write_u64(1).await?;

        Ok(())
    }

    async fn add_to_store_nar(&mut self) -> EmptyResult {
        let path = self.read_string().await?;
        //let path = std::path::PathBuf::from(&path);
        let mut path = super::store::ValidPathInfo::from(path);

        let deriver = self.read_string().await?;
        let deriver = if deriver == "" {
            None
        } else {
            Some(
                self.store
                    .print_store_path(&std::path::PathBuf::from(deriver))?,
            )
        }
        .map(|v| std::path::PathBuf::from(v));
        path.deriver = deriver;

        debug!("add {} to store", path);

        //path.nar_hash = super::store::Hash::sha256(self.read_string().await?);
        path.nar_hash = super::store::Hash::from_sha256(&self.read_string().await?)?;
        // read references
        let references = self.read_strings().await?;
        let references: Vec<std::path::PathBuf> = references
            .into_iter()
            .map(move |v| std::path::PathBuf::from(v))
            .collect();
        path.references = references;
        path.registration_time =
            chrono::NaiveDateTime::from_timestamp(self.read_int().await? as i64, 0);
        path.nar_size = Some(self.read_int().await?);
        path.ultimate = self.read_int().await? != 0;
        path.sigs = self.read_strings().await?;
        path.ca = Some(self.read_string().await?); // TODO: better type

        let repair = self.read_int().await? != 0;
        let mut dont_check_sigs = self.read_int().await? != 0;
        if !self.trusted && dont_check_sigs {
            dont_check_sigs = false;
        }
        if !self.trusted {
            path.ultimate = false;
        }

        self.logger.start_work().await?;

        self.store
            .add_to_store(path, /*source,*/ repair, !dont_check_sigs)
            .await?;
        self.logger.stop_work(logger::WORKDONE).await?;

        Ok(())
    }

    #[allow(dead_code, unused_assignments, unused_variables)]
    async fn add_to_store(&mut self) -> EmptyResult {
        let base_name = self.read_string().await?;
        let fixed = self.read_int().await? != 0; // obsolete?
        let methode = self.read_int().await?;
        use std::convert::TryFrom;
        let mut methode = super::store::FileIngestionMethod::try_from(methode)?;
        let mut s = self.read_string().await?;

        trace!("adding {} to store", base_name);

        // Compatibility hack
        if !fixed {
            s = "sha256".to_string();
            methode = super::store::FileIngestionMethod::Recursive;
        }

        self.logger.start_work().await?;

        let hash = self.parse_dump(&base_name).await?;
        // TODO: move path into store
        // How is the Hash calculated? from fixed output?
        warn!("get hash");

        self.logger.stop_work(logger::WORKDONE).await?;
        // return store path to nix client
        warn!("return path");
        warn!("hash: {}", hash);
        // TODO: add to sql database
        self.write_string(&hash.to_string()).await?; // TODO: rename to path

        Ok(())
    }

    async fn add_text_to_store(&mut self) -> EmptyResult {
        let suffix = self.read_string().await?;
        let s = self.read_os_string().await?;

        let refs = self.read_strings().await?;

        self.logger.start_work().await?;
        let path = self
            .store
            .add_text_to_store(&suffix, &s, &refs, false)
            .await?;
        self.logger.stop_work(logger::WORKDONE).await?;

        self.write_string(&path.to_string()).await?;

        Ok(())
    }

    async fn build_paths(&mut self) -> EmptyResult {
        let drvs = self.read_strings().await?;
        info!("building pathes: \n{}", drvs.join("\n"));

        let mode = self.read_int().await?;
        trace!("using mode: {}", mode);

        self.logger.start_work().await?;
        warn!("build pathes");
        self.logger.stop_work(logger::WORKDONE).await?;

        self.write_u64(1).await?;

        Ok(())
    }

    async fn ensure_path(&mut self) -> EmptyResult {
        let path = self.read_string().await?;
        trace!("ensure path {}", path);

        self.logger.start_work().await?;
        //self.store.ensure_path(path).await?; // TODO: implement
        self.logger.stop_work(logger::WORKDONE).await?;

        self.write_u64(1).await?;
        Ok(())
    }

    pub async fn parse_dump(
        &mut self,
        path: &str,
    ) -> Result<super::store::ValidPathInfo, StoreError> {
        use super::store::ValidPathInfo;

        let store_dir = self.store.get_store_dir().await?;
        let extract_file = format!("{}/.temp/{}", store_dir, path);
        self.store
            .delete_path(&std::path::PathBuf::from(&extract_file))
            .await?;

        if let Some(v) = std::path::Path::new(&extract_file).parent() {
            // only create parent incase we are just a file
            std::fs::create_dir_all(v)?;
        }

        let mut reader = self.reader.write().unwrap();
        let parser = crate::archive::NarParser::new(
            &extract_file,
            &mut *reader,
            self.store.box_clone_write(),
        );
        let parser = parser.parse().await.unwrap();
        drop(reader);

        let hash_compressed = parser.hash.clone();
        let hash_compressed = hash_compressed.compress_hash(20)?;
        let result = format!(
            "{}/{}-{}",
            self.store.get_store_dir().await?,
            hash_compressed,
            path
        )
        .replace("//", "/"); // TODO: make cleaner
        let result = std::path::PathBuf::from(result);
        //let result = result.canonicalize()?;

        //std::fs::remove_dir_all(&result);
        self.store
            .delete_path(&std::path::PathBuf::from(&result))
            .await?;

        std::fs::rename(extract_file, &result)?; // TODO: will alsway have localStore?

        let result = ValidPathInfo::now(&result.display().to_string(), parser.hash, parser.len)?;
        let result = self.store.register_path(result).await?;

        Ok(result)
    }

    pub async fn update_hasher(&self, data: &[u8]) -> EmptyResult {
        let mut hasher = self.hasher.write().unwrap();
        if let Some(v) = &mut *hasher {
            v.1 += data.len();
            v.0.update(data);
        }

        Ok(())
    }

    // TODO: maybe implement own Async{Read,Write}Ext
    async fn read_int(&self) -> Result<u64, StoreError> {
        let mut reader = self.reader.write().unwrap();
        let mut buf: [u8; 8] = [0; 8];

        reader.read_exact(&mut buf).await?;
        self.update_hasher(&buf).await?;

        Ok(LittleEndian::read_u64(&buf))
    }

    // TODO: maybe implement own Async{Read,Write}Ext
    async fn write_u64(&self, v: u64) -> EmptyResult {
        trace!("write the number {}", v);
        let mut buf: [u8; 8] = [0; 8];
        LittleEndian::write_u64(&mut buf, v);

        let mut writer = self.writer.write().unwrap();
        writer.write(&buf).await?;

        Ok(())
    }

    async fn write_bool(&self, v: bool) -> EmptyResult {
        if v {
            self.write_u64(1).await
        } else {
            self.write_u64(0).await
        }
    }

    async fn read_os_string(&self) -> Result<Vec<u8>, StoreError> {
        let mut len = self.read_int().await?;

        let mut buf: [u8; 1024] = [0; 1024];
        let mut value = Vec::new();
        let mut reader = self.reader.write().unwrap();

        while len > 1024 {
            reader.read_exact(&mut buf).await?;
            value.extend_from_slice(&buf);
            len = len - 1024;
        }

        reader.read_exact(&mut buf[..len as usize]).await?;
        value.extend_from_slice(&buf[..len as usize]);
        drop(reader);

        self.update_hasher(&value).await?;
        self.read_padding(len).await?;

        Ok(value)
    }

    async fn read_string(&self) -> Result<String, StoreError> {
        Ok(String::from_utf8_lossy(&self.read_os_string().await?).to_string())
    }

    async fn write_string(&self, str: &str) -> EmptyResult {
        let len = str.len();

        trace!("writing string '{}' with len {}", str, len);

        self.write_u64(len as u64).await?;

        let mut writer = self.writer.write().unwrap();
        writer.write(str.as_bytes()).await?;
        drop(writer);

        self.write_padding(len).await?;

        Ok(())
    }

    async fn read_strings(&self) -> Result<Vec<String>, StoreError> {
        let len = self.read_int().await?;

        let mut vec = Vec::with_capacity(len as usize);
        for _v in 0..len {
            vec.push(self.read_string().await?);
        }

        Ok(vec)
    }

    async fn write_strings(&self, v: &Vec<String>) -> EmptyResult {
        self.write_u64(v.len() as u64).await?;

        for v in v {
            self.write_string(v).await?;
        }

        Ok(())
    }

    async fn read_padding(&self, len: u64) -> EmptyResult {
        if len % 8 != 0 {
            let mut buf: [u8; 8] = [0; 8];
            let len = 8 - (len % 8) as usize;

            let mut reader = self.reader.write().unwrap();
            reader.read_exact(&mut buf[0..len]).await?;
            self.update_hasher(&buf[0..len]).await?;
            // TODO: check for non 0 values
        }
        Ok(())
    }

    async fn write_padding(&self, len: usize) -> EmptyResult {
        if len % 8 != 0 {
            let buf: [u8; 8] = [0; 8];
            let len = 8 - (len % 8);
            trace!("write a padding of {} zeros", len);

            let mut writer = self.writer.write().unwrap();
            writer.write(&buf[0..len]).await?;
        }

        Ok(())
    }
}

// This trivial implementation of `drop` adds a print to console.
impl<'a> Drop for Connection<'a> {
    fn drop(&mut self) {
        //println!("> Dropping {}", self.name);
        debug!("dropping Connecton");
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

pub enum State {
    None,
    File(tokio::fs::File),
}
