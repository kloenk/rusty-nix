use log::*;

// for async trait

/// These are exported, because there are needed for async traits
pub use futures::future::LocalFutureObj;
/// These are exported, because there are needed for async traits
pub use std::boxed::Box;
use std::sync::Arc;

pub use crate::error::StoreError;

use chrono::NaiveDateTime;

pub mod local_store;
pub mod protocol;

pub mod path;

pub use path::StorePath;

/// This is a store backend wich does not save things onto db
#[cfg(test)]
pub mod mock_store;

mod valid_path;
pub use valid_path::ValidPathInfo;

mod hash;
pub use hash::Hash;

#[derive(Debug)]
pub struct MissingInfo {
    pub done: Vec<String>,

    pub will_build: path::StorePaths,
    pub will_substitute: path::StorePaths,
    pub unknown: path::StorePaths,
    pub download_size: u64,
    pub nar_size: u64,
}

impl MissingInfo {
    pub fn new() -> Self {
        Self {
            nar_size: 0,
            download_size: 0,

            will_build: Vec::new(),
            will_substitute: Vec::new(),
            unknown: Vec::new(),

            done: Vec::new(),
        }
    }
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

            println!("missing: {:?}", missing);

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

    fn box_clone_build(&self) -> Box<dyn BuildStore>;
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
        refs: &'a path::StorePaths,
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
        refs: &'a path::StorePaths,
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

    fn make_fixed_output_path<'a>(
        &'a self,
        methode: FileIngestionMethod,
        hash: &'a Hash,
        name: &'a str,
        refs: &'a path::StorePaths,
        has_self_ref: bool,
    ) -> LocalFutureObj<'a, Result<StorePath, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if hash.is_sha256() && methode == FileIngestionMethod::Recursive {
                self.make_store_path(&self.make_type("source", refs, has_self_ref), hash, name)
                    .await
            } else {
                assert!(refs.is_empty()); // TODO: use own assert to not panic
                let hash_2 = Hash::hash_string_sha256(&format!(
                    "fixed:out:{}{}:",
                    if methode == FileIngestionMethod::Recursive {
                        "r:"
                    } else {
                        ""
                    },
                    hash.to_sql_string()
                ))?;
                self.make_store_path("output:out", &hash_2, name).await
            }
        }))
    }

    fn make_type(&self, path_type: &str, refs: &path::StorePaths, has_self_ref: bool) -> String {
        let mut res = String::from(path_type);
        for v in refs {
            res.push(':');
            res.push_str(&self.print_store_path(v)); // TODO: use self.printStorePath?
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
        LocalFutureObj::new(Box::new(async move {
            let s = format!(
                "{}:{}:{}:{}",
                file_type,
                hash.to_sql_string(),
                self.get_store_dir()?,
                name
            ); // TODO: remove sha256
            trace!("store path hasher string is: '{}'", s);
            let hash = Hash::hash_string(&s)?;
            let hash = hash.compress_hash(20)?;

            //let s = format!("{}/{}-{}", self.get_store_dir()?, hash, name);

            //Ok(StorePath::new(&format!("{}-{}", hash.to_base32()?, name))?)
            Ok(StorePath::new_hash(hash, name)?)
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

#[derive(Debug, PartialEq, Eq)]
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
