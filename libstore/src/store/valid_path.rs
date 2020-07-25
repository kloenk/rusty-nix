use chrono::NaiveDateTime;

use super::{path, Hash, Store, StoreError, StorePath};

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

    /// Info for a binary store
    pub binary_info: Option<BinaryPathInfo>,
}

impl ValidPathInfo {
    pub fn new(path: StorePath) -> ValidPathInfo {
        Self {
            path,
            deriver: None,
            nar_hash: Hash::None,
            references: Vec::new(),
            registration_time: chrono::NaiveDateTime::from_timestamp(0, 0), // TODO: ??
            nar_size: None,
            id: 0,
            ultimate: false,
            sigs: Vec::new(),
            ca: None,
            binary_info: None,
        }
    }

    pub fn parse_str(str: &str, store: &dyn Store) -> Result<ValidPathInfo, StoreError> {
        let mut lines = str.lines();
        let store_path: Vec<&str> = lines
            .next()
            .map(|v| v.split(": ").map(|v| v.trim()).collect())
            .unwrap_or_default();
        println!("store_path: {:?}", store_path);
        if store_path.len() != 2 || !store_path[0].eq("StorePath") {
            unreachable!() // FIXME: better error
                           //return Err();
        }

        let store_path = store_path[1];
        let store_path = store.parse_store_path(store_path)?;

        let mut ret = Self::new(store_path);
        let mut bin_info = BinaryPathInfo::new();

        for l in lines {
            let parts: Vec<&str> = l.split(": ").map(|v| v.trim()).collect();
            if parts.len() == 0 {
                continue;
            } else if parts.len() != 2 {
                log::warn!("invaid narInfo parts: {:?}", parts);
                unimplemented!()
            }
            log::trace!("narInfo: {}: {}", parts[0], parts[1]);
            use std::convert::TryFrom;
            match parts[0] {
                "NarHash" => ret.nar_hash = Hash::try_from(parts[1])?, //Hash::from_sql_string(parts[1])?,
                "NarSize" => ret.nar_size = parts[1].parse().ok(),
                "References" => {
                    ret.references = parts[1]
                        .split(' ')
                        .map(|v| StorePath::new(v))
                        .collect::<Result<Vec<StorePath>, StoreError>>()?
                }
                "Deriver" => ret.deriver = StorePath::new(parts[1]).ok(),
                "Sig" => ret.sigs = parts[1].split(' ').map(|v| v.to_string()).collect(),
                "URL" => bin_info.url = parts[1].to_string(),
                "FileSize" => bin_info.file_size = parts[1].parse().ok(),
                "Compression" => bin_info.compression = Compression::try_from(parts[1])?,
                "FileHash" => bin_info.file_hash = Hash::try_from(parts[1])?,
                _ => log::warn!("what is the field '{}'?", parts[0]),
            }
        }

        if !bin_info.is_empty() {
            ret.binary_info = Some(bin_info);
        }

        Ok(ret)
    }

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
            binary_info: None,
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
            binary_info: None,
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

#[derive(Debug, Clone)]
pub struct BinaryPathInfo {
    pub compression: Compression,
    pub url: String,
    pub file_hash: Hash,
    pub file_size: Option<u64>,
}

impl BinaryPathInfo {
    pub fn new() -> Self {
        Self {
            compression: Compression::None,
            url: String::new(),
            file_hash: Hash::None,
            file_size: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        if self.compression == Compression::None
            && self.url == ""
            && self.file_hash == Hash::None
            && self.file_size == None
        {
            true
        } else {
            false
        }
    }
}

impl PartialEq for BinaryPathInfo {
    fn eq(&self, other: &BinaryPathInfo) -> bool {
        self.file_hash == other.file_hash
    }
}

impl Eq for BinaryPathInfo {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Compression {
    XZ,
    None,
}

use std::convert::TryFrom;

impl TryFrom<&str> for Compression {
    type Error = StoreError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        match v {
            "xz" => Ok(Compression::XZ),
            _ => Err(StoreError::Unimplemented {
                msg: "non xz comperssion".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::StorePath;
    use super::ValidPathInfo;
    use crate::store::mock_store::MockStore;
    #[test]
    fn from_str() {
        match env_logger::try_init() {
            // this may file because of previos test
            _ => (),
        }

        let str = r#"StorePath: /nix/store/m1a7l5663wfrp36myxwmk61yc546zkqf-libnfc-1.7.1
URL: nar/0klbnh0bsy8mc2pc875dfv01n9yc8d9rrchgq1ikfmv7g0p46fbg.nar.xz
Compression: xz
FileHash: sha256:0klbnh0bsy8mc2pc875dfv01n9yc8d9rrchgq1ikfmv7g0p46fbg
FileSize: 121344
NarHash: sha256:1ym9fi02fpy694b8qzx4rzf0ysl24m1783yhj0rijl2cxjd7bbf5
NarSize: 582352
References: bqbg6hb2jsl3kvf6jgmgfdqy06fpjrrn-glibc-2.30 hky7c4s917r4sp5iff5f8vh1qdmys2z8-libusb-compat-0.1.7 m1a7l5663wfrp36myxwmk61yc546zkqf-libnfc-1.7.1
Deriver: kabgla47hqvkq7i83d46x06ljxq042zf-libnfc-1.7.1.drv
Sig: cache.nixos.org-1:k+3WgwIvPX+cgvmpY3aSj6l0zV0D3YKL+8h0DyA4aMmNfRsFJ65Z5GZQKBAA7T5BfNYlvGn9S9b78O1IqHJwDw=="#;

        let store = MockStore::new();

        let info = ValidPathInfo::parse_str(str, &store).unwrap();

        assert_eq!(
            info.path,
            StorePath::new("m1a7l5663wfrp36myxwmk61yc546zkqf-libnfc-1.7.1").unwrap()
        );
        assert_eq!(info.nar_size, Some(582352));
        assert_eq!(info.binary_info.unwrap().file_size, Some(121344));
        // TODO: check more fields
    }
}
