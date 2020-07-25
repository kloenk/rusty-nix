use std::collections::HashMap;

use futures::future::LocalFutureObj;
use std::boxed::Box;

macro_rules! MissingCap {
    ($arg:expr) => {
        LocalFutureObj::new(Box::new(async move {
            Err($crate::error::StoreError::MissingCap { cap: $arg })
        }))
    };
}

pub trait StoreOpener {
    fn open_reader<'a>(
        &'a self,
        uri: &'a str,
        params: std::collections::HashMap<String, crate::store::Param>,
    ) -> LocalFutureObj<'a, Result<Box<dyn crate::store::ReadStore>, crate::error::StoreError>>
    {
        MissingCap!(crate::store::StoreCap::Read)
    }

    fn open_builder<'a>(
        &'a self,
        uri: &'a str,
        params: std::collections::HashMap<String, crate::store::Param>,
    ) -> LocalFutureObj<'a, Result<Box<dyn crate::store::BuildStore>, crate::error::StoreError>>
    {
        MissingCap!(crate::store::StoreCap::Build)
    }
}

pub struct PluginRegistry {
    stores: HashMap<String, Box<dyn StoreOpener>>,
    // libraries: Vec<Arc<Library>> // FIXME: keep track of so files
}

impl PluginRegistry {
    pub fn new() -> Result<Self, crate::error::PluginError> {
        let mut ret = Self {
            stores: HashMap::new(),
        };

        // register default stores
        ret.stores.insert(
            "file".to_string(),
            crate::store::local_store::LocalStoreOpener::new(),
        );

        let conf = crate::CONFIG.read().unwrap();
        let conf = conf.plugin_files.clone();
        for p in &conf {
            unimplemented!("loading plugin {}", p);
        }

        Ok(ret)
    }

    pub async fn open_store_read(
        &self,
        uri: &str,
        params: std::collections::HashMap<String, crate::store::Param>,
    ) -> Result<Box<dyn crate::store::ReadStore>, crate::error::StoreError> {
        let uri: Vec<&str> = uri.split("://").collect();
        match uri.len() {
            1 => {
                if uri[0] == "auto" {
                    self.stores
                        .get("file")
                        .unwrap()
                        .open_reader(uri[0], params)
                        .await
                } else {
                    unimplemented!("non auto store");
                }
            }
            2 => {
                //self.stores.get(uri[0]).map(|v| v.open_reader(uri[1])).ok_or_else(crate::error::StoreError::NoStore{name: uri[0].to_string() })?.await
                self.stores
                    .get(uri[0])
                    .ok_or_else(|| crate::error::StoreError::NoStore {
                        name: uri[0].to_string(),
                    })?
                    .open_reader(uri[1], params)
                    .await
            }
            _ => unreachable!(),
        }
    }

    pub async fn open_store_build(
        &self,
        uri: &str,
        params: std::collections::HashMap<String, crate::store::Param>,
    ) -> Result<Box<dyn crate::store::BuildStore>, crate::error::StoreError> {
        let uri: Vec<&str> = uri.split("://").collect();
        match uri.len() {
            1 => {
                if uri[0] == "auto" {
                    self.stores
                        .get("file")
                        .unwrap()
                        .open_builder(uri[0], params)
                        .await
                } else {
                    unimplemented!("non auto store");
                }
            }
            2 => {
                //self.stores.get(uri[0]).map(|v| v.open_reader(uri[1])).ok_or_else(crate::error::StoreError::NoStore{name: uri[0].to_string() })?.await
                self.stores
                    .get(uri[0])
                    .ok_or_else(|| crate::error::StoreError::NoStore {
                        name: uri[0].to_string(),
                    })?
                    .open_builder(uri[1], params)
                    .await
            }
            _ => unreachable!(),
        }
    }
}
