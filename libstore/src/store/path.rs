use std::collections::HashMap;

use crate::error::StoreError;

pub const HASHLEN: u8 = 32;
const DRVEXTENSION: &str = ".drv";
pub const DUMMY: &str = "ffffffffffffffffffffffffffffffff-x"; // TODO: test with this as example
pub const STORE_PATH: &str = "/nix/store"; // TODO: uses non hardcoded thingi

pub type StorePaths = Vec<StorePath>;
pub type OutputPathMap = HashMap<String, StorePath>;

#[derive(Debug, Clone)]
pub struct StorePath {
    base_name: String,
}

impl StorePath {
    pub fn new(base_name: &str) -> Result<Self, crate::error::StoreError> {
        let base_name = base_name.to_string();
        if base_name.len() < (HASHLEN + 1) as usize {
            return Err(StoreError::NotInStore { path: base_name });
        }
        let path = Self { base_name };

        for v in path.hash_part().as_bytes() {
            match (*v) as char {
                'e' | 'o' | 'u' | 't' => {
                    return Err(StoreError::NotInStore {
                        path: path.base_name,
                    });
                }
                _ => continue,
            }
        }

        // TODO: check thas HASHLEN +1 is '-'?

        Ok(path)
    }

    pub fn new_hash(hash: super::Hash, name: &str) -> Result<Self, StoreError> {
        Self::new(&format!("{}-{}", hash, name))
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

#[cfg(test)]
mod test {
    use super::StorePath;
    use super::DUMMY;
    #[tokio::test]
    async fn from_store_path() {
        use crate::Store;
        let path_1 = StorePath::new(DUMMY).unwrap();
        let store = crate::store::mock_store::MockStore::new();
        let path_2 = store
            .parse_store_path(&format!("/nix/store/{}", DUMMY))
            .await
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
        let path = store.print_store_path(&path).await;

        assert_eq!(path, "/nix/store/ffffffffffffffffffffffffffffffff-x");
    }
}
