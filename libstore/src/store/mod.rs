use crate::error::StoreError;

use std::sync::{Arc, RwLock};

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
    pub nar_hash: Hash,     // TODO: type narHash
    pub references: String, // TODO: type StorePathSets
    pub registration_time: NaiveDateTime,
    pub narSize: Option<u64>,
    pub id: u64, // internal use only

    /* Whether the path is ultimately trusted, that is, it's a
    derivation output that was built locally. */
    pub ultimate: bool,

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
    /*/* Return a fingerprint of the store path to be used in binary
    cache signatures. It contains the store path, the base-32
    SHA-256 hash of the NAR serialisation of the path, the size of
    the NAR, and the sorted references. The size field is strictly
    speaking superfluous, but might prevent endless/excessive data
    attacks. */
    std::string fingerprint(const Store & store) const;

    void sign(const Store & store, const SecretKey & secretKey);

    /* Return true iff the path is verifiably content-addressed. */
    bool isContentAddressed(const Store & store) const;

    static const size_t maxSigs = std::numeric_limits<size_t>::max();

    /* Return the number of signatures on this .narinfo that were
       produced by one of the specified keys, or maxSigs if the path
       is content-addressed. */
    size_t checkSignatures(const Store & store, const PublicKeys & publicKeys) const;

    /* Verify a single signature. */
    bool checkSignature(const Store & store, const PublicKeys & publicKeys, const std::string & sig) const;

    Strings shortRefs() const;

    ValidPathInfo(const StorePath & path) : path(path) { }

    ValidPathInfo(StorePath && path) : path(std::move(path)) { }

    virtual ~ValidPathInfo() { }*/
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
    sha256(String), // TOOD: use sha256 type
}

impl std::convert::From<&str> for Hash {
    fn from(v: &str) -> Self {
        let v: Vec<&str> = v.split(':').collect();
        match *v.get(0).unwrap_or(&"") {
            "sha256" => Hash::sha256(v.get(1).unwrap().to_string()),
            _ => panic!("invalid hash"),
        }
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hash::sha256(v) => write!(f, "{}", v), // no sha256:<hash>??
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

    fn get_store_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>>;

    fn get_state_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>>;

    fn print_store_path<'a>(&'a self, p: std::path::PathBuf) -> Result<String, StoreError> {
        // TODO: C++ adds `storeDir + "/"` infront??
        Ok(p.display().to_string())
    }

    //fn print_store_paths<'a>('a self, p: Vec<>)
}

pub fn openStore(
    store_uri: &str,
    params: std::collections::HashMap<String, String>,
) -> Result<Box<dyn Store>, StoreError> {
    if store_uri == "auto" {
        let store = local_store::LocalStore::openStore("/nix/", params)?;
        return Ok(Box::new(store));
    }

    // FIXME: magic for other store bachends
    if !store_uri.starts_with("file://") {
        return Err(crate::error::StoreError::InvalidStoreUri {
            uri: store_uri.to_string(),
        });
    }

    let path = &store_uri["file://".len()..];
    let store = local_store::LocalStore::openStore(path, params)?;
    Ok(Box::new(store))
}
