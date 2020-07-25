use std::collections::HashMap;

use futures::future::LocalFutureObj;
use std::boxed::Box;

use std::rc::Rc;

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
        _uri: &'a str,
        _params: std::collections::HashMap<String, crate::store::Param>,
    ) -> LocalFutureObj<'a, Result<Box<dyn crate::store::ReadStore>, crate::error::StoreError>>
    {
        MissingCap!(crate::store::StoreCap::Read)
    }

    fn open_builder<'a>(
        &'a self,
        _uri: &'a str,
        _params: std::collections::HashMap<String, crate::store::Param>,
    ) -> LocalFutureObj<'a, Result<Box<dyn crate::store::BuildStore>, crate::error::StoreError>>
    {
        MissingCap!(crate::store::StoreCap::Build)
    }
}

struct StorePlugin {
    cap: crate::store::StoreCap,
    opener: Box<dyn StoreOpener>,
}

pub struct PluginRegistry {
    stores: HashMap<String, Rc<StorePlugin>>, // Rc for aliases
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
            Rc::new(StorePlugin {
                cap: crate::store::StoreCap::Build,
                opener: crate::store::local_store::LocalStoreOpener::new(),
            }),
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
                    // auto store is hardcoded. so It always has to be a `BuildStore`
                    self.stores
                        .get("file")
                        .unwrap()
                        .opener
                        .open_reader(uri[0], params)
                        .await
                } else {
                    unimplemented!("non auto store");
                }
            }
            2 => {
                let store =
                    self.stores
                        .get(uri[0])
                        .ok_or_else(|| crate::error::StoreError::NoStore {
                            name: uri[0].to_string(),
                        })?;
                if store.cap < crate::store::StoreCap::Read {
                    return Err(crate::error::StoreError::MissingCap {
                        cap: crate::store::StoreCap::Read,
                    });
                }
                store.opener.open_reader(uri[1], params).await
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
                    // auto store is hardcoded. so It always has to be a `BuildStore`
                    self.stores
                        .get("file")
                        .unwrap()
                        .opener
                        .open_builder(uri[0], params)
                        .await
                } else {
                    unimplemented!("non auto store");
                }
            }
            2 => {
                let store =
                    self.stores
                        .get(uri[0])
                        .ok_or_else(|| crate::error::StoreError::NoStore {
                            name: uri[0].to_string(),
                        })?;
                if store.cap < crate::store::StoreCap::Read {
                    return Err(crate::error::StoreError::MissingCap {
                        cap: crate::store::StoreCap::Read,
                    });
                }
                store.opener.open_builder(uri[1], params).await
            }
            _ => unreachable!(),
        }
    }

    pub async fn open_default_substituters(
        &self,
    ) -> Result<Vec<Box<dyn crate::store::ReadStore>>, crate::error::StoreError> {
        let settings = crate::CONFIG.read().unwrap();
        let mut ret: Vec<Box<dyn crate::store::ReadStore>> = Vec::new();
        use std::collections::HashMap;
        let empty = HashMap::new();

        for uri in &settings.substituters {
            ret.push(self.open_store_read(uri, empty.clone()).await?);
        }

        for uri in &settings.extra_substituters {
            ret.push(self.open_store_read(uri, empty.clone()).await?);
        }

        ret.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(ret)
    }
}
