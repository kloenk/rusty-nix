use std::collections::HashMap;

use log::*;

use crate::error::StoreError;

pub const HASHLEN: u8 = 32;
const DRVEXTENSION: &str = ".drv";
pub const DUMMY: &str = "ffffffffffffffffffffffffffffffff-x"; // TODO: test with this as example
pub const STORE_PATH: &str = "/nix/store"; // TODO: uses non hardcoded thingi

pub type StorePaths = Vec<StorePath>;
pub type OutputPathMap = HashMap<String, StorePath>;

#[derive(Debug, Clone, Hash)]
pub struct StorePath {
    base_name: String,
}

impl StorePath {
    /// create new StorePath from basename
    pub fn new(base_name: &str) -> Result<Self, crate::error::StoreError> {
        let base_name = base_name.to_string();
        if base_name.len() < (HASHLEN + 1) as usize {
            return Err(StoreError::NotInStore { path: base_name });
        }
        let path = Self { base_name };

        for v in path.hash_part().as_bytes() {
            match (*v) as char {
                'e' | 'o' | 'u' | 't' => {
                    warn!("create real error");
                    return Err(StoreError::InvalidHashPart {
                        path: path.base_name.clone(),
                        hash_part: path.hash_part(),
                    });
                }
                _ => continue,
            }
        }

        // TODO: check thas HASHLEN +1 is '-'?

        Ok(path)
    }

    pub fn new_hash(hash: super::Hash, name: &str) -> Result<Self, StoreError> {
        Self::new(&format!("{}-{}", hash.to_base32()?, name))
    }

    pub fn is_derivation(&self) -> bool {
        self.base_name.ends_with(DRVEXTENSION)
    }

    pub fn name(&self) -> String {
        self.base_name[(HASHLEN + 1) as usize..].to_string()
    }

    pub fn hash_part(&self) -> String {
        self.base_name[..(HASHLEN as usize)].to_string()
    }

    /*pub fn to_string<T: super::Store>(&self, store: &Box<dyn super::Store>) -> String {
        store.print_store_path(self)
    }*/
}

use std::fmt;
impl fmt::Display for StorePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base_name)
    }
}

impl PartialEq for StorePath {
    fn eq(&self, other: &Self) -> bool {
        self.base_name == other.base_name
    }
}

impl PartialEq<str> for StorePath {
    fn eq(&self, other: &str) -> bool {
        self.base_name == other
    }
}

impl Eq for StorePath {}

use std::cmp::Ordering;
impl Ord for StorePath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.base_name.cmp(&other.base_name)
    }
}

impl PartialOrd for StorePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePathWithOutputs {
    pub path: StorePath,
    pub outputs: Vec<String>,
}

impl StorePathWithOutputs {
    pub fn new(path: StorePath) -> Self {
        Self {
            path,
            outputs: Vec::new(),
        }
    }

    pub fn new_with_outputs(path: StorePath, outputs: Vec<String>) -> Self {
        Self { path, outputs }
    }

    /*pub fn to_string<T: super::Store>(&self, store: &T) -> String {
        if self.outputs.len() == 0 {
            self.path.to_string(store)
        } else {
            format!("{}!{}", self.path.to_string(store), self.outputs.join(","))
        }
    }*/
}

impl PartialEq<StorePath> for StorePathWithOutputs {
    fn eq(&self, other: &StorePath) -> bool {
        &self.path == other
    }
}

#[derive(Debug, Clone)]
pub struct SubstitutablePathInfo {
    pub deriver: Option<StorePath>,
    pub references: StorePaths,

    /// None if Unkown or inapplicable
    pub donwload_size: Option<u64>,
    /// None if Unkown
    pub nar_size: Option<u64>,
}

impl std::convert::From<super::ValidPathInfo> for SubstitutablePathInfo {
    fn from(v: super::ValidPathInfo) -> Self {
        Self {
            deriver: v.deriver,
            donwload_size: v.binary_info.map(|v| v.file_size).flatten(),
            references: v.references,
            nar_size: v.nar_size,
        }
    }
}

pub type SubstitutablePathInfos = Vec<SubstitutablePathInfo>;

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::StorePath;
    use super::DUMMY;
    #[tokio::test]
    async fn from_store_path() {
        use crate::Store;
        let path_1 = StorePath::new(DUMMY).unwrap();
        let store = crate::store::mock_store::MockStore::new();
        let store = Arc::new(store);
        let path_2 = store
            .parse_store_path(&format!("/nix/store/{}", DUMMY))
            .unwrap();

        assert_eq!(path_1, path_2);
    }

    #[test]
    fn get_hash_part() {
        let path = StorePath::new(DUMMY).unwrap();

        assert_eq!(path.hash_part(), "ffffffffffffffffffffffffffffffff");
        assert_eq!(path.name(), "x");
    }

    #[tokio::test]
    async fn print_store_path() {
        let path = StorePath::new(DUMMY).unwrap();
        use crate::Store;
        let store = crate::store::mock_store::MockStore::new();
        let store = Arc::new(store);
        let path = store.print_store_path(&path);

        assert_eq!(path, "/nix/store/ffffffffffffffffffffffffffffffff-x");
    }

    #[test]
    fn with_outputs() {
        use crate::Store;
        let store = crate::store::mock_store::MockStore::new();
        let store = Arc::new(store);

        let path = format!("{}/{}!out,dev", super::STORE_PATH, DUMMY);

        let paths = store.parse_store_path_with_outputs(&path).unwrap();
        let paths_2 = super::StorePathWithOutputs::new_with_outputs(
            StorePath::new(DUMMY).unwrap(),
            vec!["out".to_string(), "dev".to_string()],
        );

        assert_eq!(paths, StorePath::new(DUMMY).unwrap());
        assert_eq!(paths, paths_2);
    }
}
