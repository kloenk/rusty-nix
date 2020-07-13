use log::*;

// for async trait

/// These are exported, because there are needed for async traits
pub use futures::future::LocalFutureObj;
/// These are exported, because there are needed for async traits
pub use std::boxed::Box;

pub use crate::error::StoreError;

use chrono::NaiveDateTime;

pub mod local_store;
pub mod protocol;

pub mod path;

pub use path::StorePath;

/// This is a store backend wich does not save things onto db
#[cfg(test)]
pub mod mock_store;

#[derive(Debug, Clone)]
pub struct ValidPathInfo {
    pub path: StorePath,
    pub deriver: Option<StorePath>,
    pub nar_hash: Hash,               // TODO: type narHash
    pub references: path::StorePaths, // TODO: type StorePathSets
    pub registration_time: NaiveDateTime,
    pub nar_size: Option<u64>,
    pub id: u64, // internal use only

    /* Whether the path is ultimately trusted, that is, it's a
    derivation output that was built locally. */
    pub ultimate: bool,

    // TODO: maybe add a type which sepperates signer from signature
    pub sigs: Vec<String>, // not necessarily verified

    /* If non-empty, an assertion that the path is content-addressed,
       i.e., that the store path is computed from a cryptographic hash
       of the contents of the path, plus some other bits of data like
       the "name" part of the path. Such a path doesn't need
       signatures, since we don't have to trust anybody's claim that
       the path is the output of a particular derivation. (In the
       extensional store model, we have to trust that the *contents*
       of an output path of a derivation were actually produced by
       that derivation. In the intensional model, we have to trust
       that a particular output path was produced by a derivation; the
       path then implies the contents.)

       Ideally, the content-addressability assertion would just be a
       Boolean, and the store path would be computed from
       the name component, ‘narHash’ and ‘references’. However,
       1) we've accumulated several types of content-addressed paths
       over the years; and 2) fixed-output derivations support
       multiple hash algorithms and serialisation methods (flat file
       vs NAR). Thus, ‘ca’ has one of the following forms:

       * ‘text:sha256:<sha256 hash of file contents>’: For paths
         computed by makeTextPath() / addTextToStore().

       * ‘fixed:<r?>:<ht>:<h>’: For paths computed by
         makeFixedOutputPath() / addToStore().
    */
    pub ca: Option<String>,
}

impl ValidPathInfo {
    pub fn now(path: StorePath, hash: Hash, size: u64) -> Result<ValidPathInfo, StoreError> {
        use chrono::prelude::*;
        let now: DateTime<Utc> = Utc::now();
        Ok(Self {
            path,
            deriver: None,
            nar_hash: hash,
            references: Vec::new(),
            registration_time: now.naive_utc(),
            nar_size: Some(size),
            ca: None,
            id: 0,
            sigs: Vec::new(),
            ultimate: false,
        })
    }
    /// Return a fingerprint of the store path to be used in binary
    /// cache signatures. It contains the store path, the base-32
    /// SHA-256 hash of the NAR serialisation of the path, the size of
    /// the NAR, and the sorted references. The size field is strictly
    /// speaking superfluous, but might prevent endless/excessive data
    /// attacks.
    // std::string fingerprint(const Store & store) const;
    pub fn fingerprint(&self, store: &Box<dyn Store>) -> Result<String, StoreError> {
        if (self.nar_size == None || self.nar_size.unwrap() == 0) || self.nar_hash == Hash::None {
            return Err(StoreError::NoFingerprint {
                path: self.path.to_string(),
            });
        }

        // nar hash to Base32
        let mut nar_hash = String::new();
        if let Hash::SHA256(v) = &self.nar_hash {
            nar_hash = data_encoding::BASE32.encode(v)
        } // TODO: make pretty

        Ok(format!(
            "1;{};{};{};{}",
            store.print_store_path(&self.path),
            nar_hash,
            self.nar_size.unwrap(),
            self.references
                .iter()
                .map(|v| store.print_store_path(v))
                .collect::<Vec<String>>()
                .join(",")
        ))
    }
    /*

    void sign(const Store & store, const SecretKey & secretKey);

    /* Return true iff the path is verifiably content-addressed. */
    bool isContentAddressed(const Store & store) const;

    static const size_t maxSigs = std::numeric_limits<size_t>::max();
    */
    /// Return the number of signatures on this .narinfo that were
    /// produced by one of the specified keys, or maxSigs if the path
    /// is content-addressed.
    //size_t checkSignatures(const Store & store, const PublicKeys & publicKeys) const;
    pub fn check_signatures(&self, store: &Box<dyn Store>) -> Result<usize, StoreError> {
        // TODO: ca foo

        use crate::crypto::PublicKeys;
        use std::convert::TryFrom;
        let config = crate::CONFIG.read().unwrap();
        let public_keys = PublicKeys::try_from(config.trusted_public_keys.clone())?;
        drop(config);

        let mut good = 0;
        for v in &self.sigs {
            if self.check_signature(&v, &public_keys, store)? {
                good += 1;
            }
        }

        Ok(good)
    }

    ///Verify a single signature.
    //bool checkSignature(const Store & store, const PublicKeys & publicKeys, const std::string & sig) const;
    pub fn check_signature(
        &self,
        sig: &str,
        public_keys: &crate::crypto::PublicKeys,
        store: &Box<dyn Store>,
    ) -> Result<bool, StoreError> {
        public_keys.verify(self.fingerprint(store)?.as_bytes(), sig)
    }

    /*
    Strings shortRefs() const;*/
}

#[deprecated = "use try-from version"]
impl std::convert::From<String> for ValidPathInfo {
    fn from(v: String) -> Self {
        Self {
            path: StorePath::new(&v).unwrap(),
            deriver: None,
            nar_hash: Hash::None,
            references: Vec::new(),
            registration_time: chrono::NaiveDateTime::from_timestamp(0, 0), // TODO: ??
            nar_size: None,
            id: 0,
            ultimate: false,
            sigs: Vec::new(),
            ca: None,
        }
    }
}

// TODO: build try-from version for Validpath from String

impl std::fmt::Display for ValidPathInfo {
    /// This only returns a path.
    // TODO: maby add an extra type which makes a more verbose output with usage like std::path::PathBuf.display()
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: more verbose output?
        write!(f, "{}", self.path)
    }
}

impl PartialEq for ValidPathInfo {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.nar_hash == other.nar_hash
            && self.references == other.references
    }
}
impl Eq for ValidPathInfo {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Hash {
    SHA256([u8; 32]),
    Compressed(Vec<u8>),
    None,
}

impl Hash {
    pub fn from_sha256(v: &str) -> Result<Self, StoreError> {
        let mut buf: [u8; 32] = [0; 32];
        data_encoding::HEXLOWER
            .decode_mut(v.as_bytes(), &mut buf)
            .map_err(|_| StoreError::HashDecodePartialError {
                error: v.to_string(),
            })?; // TODO: error handling
                 //base64::decode_config_slice(v, base64::STANDARD, &mut buf)?;
        Ok(Hash::SHA256(buf))
    }
    pub fn is_sha256(&self) -> bool {
        match self {
            Hash::SHA256(_) => true,
            _ => false,
        }
    }
    pub fn from_sha256_vec(v: &[u8]) -> Result<Self, StoreError> {
        let mut buf: [u8; 32] = [0; 32];
        buf.copy_from_slice(v); // TODO: no panicing
        Ok(Hash::SHA256(buf))
    }

    pub fn to_base32(&self) -> Result<String, StoreError> {
        match self {
            Hash::SHA256(v) => {
                let v = data_encoding::BASE32.encode(v);
                Ok(v)
            }
            _ => Err(StoreError::BadArchive {
                msg: "base32 error".to_string(),
            }), // TODO: better error type
        }
    }

    pub fn compress_hash(self, len: usize) -> Result<Self, StoreError> {
        // TODO: only take a referenc, so no cloning is needed? (or even return another type?)
        //let mut vec=  Vec::with_capacity(len);
        let mut vec = vec![0; len];
        if let Hash::SHA256(v) = self {
            for i in 0..v.len() {
                vec[i % len] ^= v[i];
            }
        } else {
            return Err(StoreError::Unimplemented {
                msg: "compress_hash".to_string(),
            });
        }
        Ok(Hash::Compressed(vec))
    }

    pub fn to_sql_string(&self) -> String {
        // TODO: return StoreError for none?
        match self {
            Hash::SHA256(v) => format!("sha256:{}", data_encoding::HEXLOWER.encode(v)),
            _ => "unsuported".to_string(),
        }
    }

    pub fn hash_string(s: &str) -> Result<Hash, StoreError> {
        // read hash type from s
        trace!("reading hash string: '{}'", s);
        let ht: Vec<&str> = s.split(':').collect();
        if ht.len() < 4 {
            unimplemented!("invalid hash string");
        }
        let ht_pos = ht.len() - 4;
        let ht = ht[ht_pos];
        match ht {
            "sha256" => Hash::from_sha256_vec(
                ring::digest::digest(&ring::digest::SHA256, s.as_bytes()).as_ref(),
            ),
            _ => unimplemented!("not sha256"),
        }
    }
}

impl std::convert::From<&str> for Hash {
    fn from(v: &str) -> Self {
        trace!("making hash from '{}'", v);
        let v: Vec<&str> = v.split(':').collect();
        match *v.get(0).unwrap_or(&"") {
            "sha256" => {
                //let mut buf: [u8; 32] = [0; 32];
                //trace!("decoding sha hash: {}", v.get(1).unwrap());
                //base64::decode_config_slice(v.get(1).unwrap(), base64::STANDARD, &mut buf).unwrap(); // TODO: error handling
                //Hash::sha256(v.get(1).unwrap().to_string())
                let mut buf: [u8; 32] = [0; 32];
                trace!("decoding sha hash: {}", v.get(1).unwrap());
                data_encoding::HEXLOWER
                    .decode_mut(v.get(1).unwrap().as_bytes(), &mut buf)
                    .map_err(|_| StoreError::HashDecodePartialError {
                        error: v.get(1).unwrap().to_string(),
                    })
                    .unwrap(); // TODO: error handling
                Hash::SHA256(buf)
            }
            _ => panic!("invalid hash"),
        }
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hash::SHA256(v) => write!(f, "{}", data_encoding::HEXLOWER.encode(v)), // no sha256:<hash>??
            Hash::None => write!(f, "EMTPY-HASH"),
            Hash::Compressed(v) => {
                let mut spec = data_encoding::Specification::new();
                spec.symbols.push_str("0123456789abcdfghijklmnpqrsvwxyz"); // TODO: make global version
                let s = spec.encoding().unwrap().encode(v);
                write!(f, "{}", s)
                /*let s = data_encoding::BASE32_NOPAD.encode(v).to_ascii_lowercase(); write!(f, "{}", s)*/
            }
        }
    }
}

pub struct MissingInfo {
    pub will_build: path::StorePaths,
    pub will_substitute: path::StorePaths,
    pub unknown: path::StorePaths,
    pub download_size: u64,
    pub nar_size: u64,
}

pub trait BuildStore: WriteStore + ReadStore + Store {
    fn build_paths<'a>(
        &'a self,
        drvs: Vec<path::StorePathWithOutputs>,
        mode: u8,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>; // TODO: make mode an enum

    fn query_missing<'a>(
        &'a self,
        paths: &'a Vec<path::StorePathWithOutputs>,
    ) -> LocalFutureObj<'a, Result<MissingInfo, StoreError>>;

    fn prime_cache<'a>(
        &'a self,
        drvs: &'a Vec<path::StorePathWithOutputs>,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            let missing = self.query_missing(drvs).await?;

            let conf = crate::CONFIG.read().unwrap();
            let max_build_jobs = conf.max_jobs.clone();
            let max_build_jobs = max_build_jobs.parse::<usize>().unwrap_or(0); // TODO: handle other cases
            drop(conf);

            if missing.will_build.len() != 0 && max_build_jobs == 0
            /* getMachines() */
            {
                return Err(StoreError::NoBuildJobs {
                    jobs: missing.will_build.len(),
                });
            }
            Ok(())
        }))
    }
}

pub trait WriteStore: ReadStore + Store {
    fn write_file<'a>(
        &'a self,
        path: &'a str,
        data: &'a [u8],
        executable: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn add_text_to_store<'a>(
        &'a self,
        suffix: &'a str,
        data: &'a [u8],
        refs: &'a Vec<String>,
        repair: bool,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>>;

    fn make_directory<'a>(&'a self, path: &str) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn make_symlink<'a>(
        &'a self,
        source: &'a str,
        target: &'a str,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>; /*{
                                                         let source = source.to_owned();
                                                         let target = target.to_owned();
                                                         LocalFutureObj::new(Box::new(async move {
                                                             Err(StoreError::Unimplemented {
                                                                 msg: format!("make_symlink: '{} -> {}'", source, target),
                                                             })
                                                         }))
                                                     }*/

    fn delete_path<'a>(&'a self, path: &'a StorePath)
        -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn register_path<'a>(
        &'a self,
        info: ValidPathInfo,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>>;

    fn add_temp_root<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn add_to_store<'a>(
        &'a self,
        //source,
        path: ValidPathInfo,
        repair: bool,
        check_sigs: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn create_user<'a>(
        &'a self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn box_clone_write(&self) -> Box<dyn WriteStore>;
}

pub trait ReadStore: Store {
    fn query_path_info<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>>;

    fn is_valid_path<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<bool, StoreError>>;

    fn make_text_path<'a>(
        &'a self,
        suffix: &'a str,
        hash: &'a Hash,
        refs: &'a Vec<String>,
    ) -> LocalFutureObj<'a, Result<StorePath, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if !hash.is_sha256() {
                return Err(StoreError::MissingHash {
                    path: "wrong hash".to_string(),
                });
            }
            let path_type = self.make_type("text", refs, false); // TODO: is this realy false?
            self.make_store_path(&path_type, hash, suffix).await
        }))
    }

    fn make_type(&self, path_type: &str, refs: &Vec<String>, has_self_ref: bool) -> String {
        let mut res = String::from(path_type);
        for v in refs {
            res.push(':');
            res.push_str(v); // TODO: use self.printStorePath?
        }
        if has_self_ref {
            res.push_str(":self");
        }

        res
    }

    fn make_store_path<'a>(
        &'a self,
        file_type: &'a str,
        hash: &'a Hash,
        name: &'a str,
    ) -> LocalFutureObj<'a, Result<StorePath, StoreError>> {
        // TODO: ValidPath as result?
        LocalFutureObj::new(Box::new(async move {
            /*/*let s = format!(
                "{}:{}:{}:{}",
                file_type,
                hash.to_string(), // TODO: is this the correct type?
                self.get_store_dir().await?,
                name
            );
            //let s = self.compressHash(HashString::SHA256(s), 20);*/
            // TODO: why does nix upstream goes via the hashString, instead direct Hash?
            let hash = hash.compress_hash(20)?;
            //let s = hash.to_string();
            let s = format!("{}/{}-{}", self.get_store_dir().await?, hash, name);
            warn!("writing to {}", s);*/
            let s = format!(
                "{}:{}:{}:{}",
                file_type,
                hash.to_sql_string(),
                self.get_store_dir()?,
                name
            ); // TODO: remove sha256
            let hash = Hash::hash_string(&s)?;
            let hash = hash.compress_hash(20)?;

            //let s = format!("{}/{}-{}", self.get_store_dir()?, hash, name);

            Ok(StorePath::new(&format!("{}-{}", hash, name))?)
        }))
    }

    fn box_clone_read(&self) -> Box<dyn ReadStore>;
}

/// This is the main store Trait, needed by every kind of store.
/// Every Store has to implement Clone. If a store has some data which in mutable inside it has to handle it itself.
pub trait Store {
    fn get_store_dir<'a>(&'a self) -> Result<String, StoreError>;

    fn get_state_dir<'a>(&'a self) -> Result<String, StoreError>;

    fn parse_store_path<'a>(&'a self, path: &'a str) -> Result<StorePath, StoreError> {
        // TODO: canon path
        let path = std::path::Path::new(path);
        let p = path.parent();
        if p.is_none() || p.unwrap() != std::path::Path::new(&self.get_store_dir()?) {
            return Err(StoreError::NotInStore {
                path: path.display().to_string(),
            });
        }
        StorePath::new(path.file_name().unwrap().to_str().unwrap())
    }

    fn parse_store_path_with_outputs<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<path::StorePathWithOutputs, StoreError> {
        let parts: Vec<&str> = path.split("!").collect();
        if parts.len() > 2 {
            return Err(StoreError::NotInStore {
                path: path.to_string(),
            }); // TODO: own error type
        }

        let path = self.parse_store_path(parts[0])?;
        let mut outputs = Vec::new();
        if parts.len() == 2 {
            outputs = parts[1].split(",").map(|v| v.to_string()).collect();
        }

        Ok(path::StorePathWithOutputs { path, outputs })
    }

    fn print_store_path<'a>(&'a self, path: &'a StorePath) -> String {
        format!("{}/{}", self.get_store_dir().unwrap(), path)
    }

    fn box_clone(&self) -> Box<dyn Store>;
}

pub async fn open_store(
    store_uri: &str,
    params: std::collections::HashMap<String, Param>,
) -> Result<Box<dyn BuildStore>, StoreError> {
    if store_uri == "auto" {
        let store = local_store::LocalStore::open_store("/nix/", params).await?;
        return Ok(Box::new(store));
    }

    // TODO: magic for other store bachends
    if !store_uri.starts_with("file://") {
        return Err(crate::error::StoreError::InvalidStoreUri {
            uri: store_uri.to_string(),
        });
    }

    let path = &store_uri["file://".len()..];
    let store = local_store::LocalStore::open_store(path, params).await?;
    Ok(Box::new(store))
}

/*pub fn print_store_path(v: &std::path::Path) -> String {
    // TODO: storeDir +
    v.display().to_string()
}*/

#[derive(Debug)]
#[repr(u8)]
pub enum FileIngestionMethod {
    Flat = 0,
    Recursive = 1,
}

impl std::convert::TryFrom<u64> for FileIngestionMethod {
    type Error = StoreError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FileIngestionMethod::Flat),
            1 => Ok(FileIngestionMethod::Recursive),
            _ => Err(StoreError::InvalidFileIngestionMethode {
                methode: value as u8,
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Param {
    String(String),
    Bool(bool),
    UInt(usize),
    Vec(Vec<Param>),
}

impl std::convert::From<String> for Param {
    fn from(v: String) -> Self {
        Param::String(v)
    }
}

impl std::convert::From<bool> for Param {
    fn from(v: bool) -> Self {
        Param::Bool(v)
    }
}

impl std::convert::From<usize> for Param {
    fn from(v: usize) -> Self {
        Param::UInt(v)
    }
}

impl<T: std::convert::Into<Param>> std::convert::From<Vec<T>> for Param {
    fn from(v: Vec<T>) -> Self {
        let mut vec = Vec::new();
        for v in v {
            vec.push(v.into());
        }
        Param::Vec(vec)
    }
}
