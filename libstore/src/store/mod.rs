use crate::error::StoreError;

use std::sync::{Arc, RwLock};

use log::{debug, info, trace, warn};

// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

use chrono::NaiveDateTime;

pub mod local_store;
pub mod protocol;

#[derive(Debug)]
pub struct ValidPathInfo {
    pub path: std::path::PathBuf,
    pub deriver: Option<std::path::PathBuf>,
    pub nar_hash: Hash,                      // TODO: type narHash
    pub references: Vec<std::path::PathBuf>, // TODO: type StorePathSets
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
    /// Return a fingerprint of the store path to be used in binary
    /// cache signatures. It contains the store path, the base-32
    /// SHA-256 hash of the NAR serialisation of the path, the size of
    /// the NAR, and the sorted references. The size field is strictly
    /// speaking superfluous, but might prevent endless/excessive data
    /// attacks.
    // std::string fingerprint(const Store & store) const;
    pub fn fingerprint(&self) -> Result<String, StoreError> {
        if (self.nar_size == None || self.nar_size.unwrap() == 0) || self.nar_hash == Hash::None {
            return Err(StoreError::NoFingerprint {
                path: self.path.display().to_string(),
            });
        }

        // nar hash to Base32
        let mut nar_hash = String::new();
        if let Hash::SHA256(v) = &self.nar_hash {
            nar_hash = data_encoding::BASE32.encode(v)
        } // TODO: make pretty

        Ok(format!(
            "1;{};{};{};{}",
            print_store_path(&self.path),
            nar_hash,
            self.nar_size.unwrap(),
            self.references
                .iter()
                .map(|v| print_store_path(&v))
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
    pub fn check_signatures(&self) -> Result<usize, StoreError> {
        // TODO: ca foo

        use crate::crypto::PublicKeys;
        use std::convert::TryFrom;
        let config = crate::CONFIG.read().unwrap();
        let public_keys = PublicKeys::try_from(config.trusted_public_keys.clone())?;
        drop(config);

        let mut good = 0;
        for v in &self.sigs {
            if self.check_signature(&v, &public_keys)? {
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
    ) -> Result<bool, StoreError> {
        let fingerprint = self.fingerprint()?;

        public_keys.verify(self.fingerprint()?.as_bytes(), sig)
    }

    /*
    Strings shortRefs() const;*/
}

impl std::convert::From<String> for ValidPathInfo {
    fn from(v: String) -> Self {
        Self {
            path: std::path::PathBuf::from(&v),
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

impl std::fmt::Display for ValidPathInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: more verbose output?
        write!(f, "validPath:{}", self.path.display())
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

#[derive(Debug, Eq, PartialEq)]
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
}

impl std::convert::From<&str> for Hash {
    fn from(v: &str) -> Self {
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

pub trait Store {
    fn create_user<'a>(
        &'a mut self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn query_path_info<'a>(
        &'a mut self,
        path: std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>>;

    fn is_valid_path<'a>(
        &'a mut self,
        path: &'a std::path::Path,
    ) -> LocalFutureObj<'a, Result<bool, StoreError>>;

    fn delete_path<'a>(
        &'a mut self,
        path: &std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn add_to_store<'a>(
        &'a mut self,
        //source,
        path: ValidPathInfo,
        repair: bool,
        check_sigs: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn add_temp_root<'a>(
        &'a mut self,
        path: &std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;

    fn make_store_path<'a>(
        &'a mut self,
        file_type: &'a str,
        hash: Hash,
        name: &'a str,
    ) -> LocalFutureObj<'a, Result<String, StoreError>> {
        // TODO: ValidPath as result?
        LocalFutureObj::new(Box::new(async move {
            /*let s = format!(
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
            warn!("writing to {}", s);

            Ok(s)
        }))
    }

    fn get_store_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>>;

    fn get_state_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>>;

    fn print_store_path<'a>(&'a self, p: &'a std::path::Path) -> Result<String, StoreError> {
        // TODO: C++ adds `storeDir + "/"` infront??
        Ok(p.display().to_string())
    }

    //fn print_store_paths<'a>('a self, p: Vec<>)
}

pub async fn open_store(
    store_uri: &str,
    params: std::collections::HashMap<String, Param>,
) -> Result<Box<dyn Store>, StoreError> {
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

pub fn print_store_path(v: &std::path::Path) -> String {
    // TODO: storeDir +
    v.display().to_string()
}

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

#[derive(Debug)]
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
