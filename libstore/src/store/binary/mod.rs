use super::{ReadStore, Store, StoreError, StorePath, ValidPathInfo};

use futures::future::LocalFutureObj;
use std::boxed::Box;

use std::sync::Arc;

use log::*;

use reqwest::Client;

pub struct BinaryStoreOpener {}

impl BinaryStoreOpener {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl crate::plugin::StoreOpener for BinaryStoreOpener {
    fn open_reader<'a>(
        &'a self,
        kind: &'a str,
        uri: &'a str,
        params: std::collections::HashMap<String, crate::store::Param>,
    ) -> LocalFutureObj<'a, Result<Box<dyn crate::store::ReadStore>, crate::error::StoreError>>
    {
        LocalFutureObj::new(Box::new(async move {
            let uri = format!("{}://{}", kind, uri);
            let store = Box::new(BinaryStore::open_store(&uri, params).await?);

            Ok(store as Box<dyn ReadStore>)
        }))
    }
}

pub struct BinaryStore {
    store_dir: String,
    priority: u64,

    params: std::collections::HashMap<String, super::Param>,

    client: Client,
    base_uri: String,
}

impl BinaryStore {
    pub async fn open_store(
        base_uri: &str,
        params: std::collections::HashMap<String, super::Param>,
    ) -> Result<Arc<Self>, StoreError> {
        let mut base_uri = base_uri.to_string();
        if !base_uri.ends_with("/") {
            base_uri.push('/');
        }
        info!("opening BinaryStore at {}", base_uri);
        trace!("got params for binarystore: {:?}", params);

        //let client = Client::new();
        let client = Client::builder();
        let client = client.user_agent(format!("nix/{}", env!("CARGO_PKG_VERSION")));
        let client = client.timeout(std::time::Duration::from_secs(5)); // FIXME: config

        let client = client.build()?;

        let uri = format!("{}nix-cache-info", base_uri);
        let response = client.get(&uri).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            warn!(
                "got non nice status coder for '{}': {}",
                uri,
                response.status()
            );
            return Err(StoreError::NoStore { name: base_uri });
        }

        let body = response.text().await?;

        let mut priority = 0;
        let mut store_dir = String::new();
        for line in body.lines() {
            let parts: Vec<&str> = line.split(':').map(|v| v.trim()).collect();
            if parts.len() != 2 {
                continue;
            }
            match parts[0] {
                "StoreDir" => store_dir = parts[1].to_string(),
                "Priority" => priority = parts[1].parse::<u64>().unwrap_or(100),
                _ => (),
            }
            trace!("{}: {}", parts[0], parts[1])
        }

        let store = Self {
            base_uri,
            client,
            params,
            priority,
            store_dir,
        };

        Ok(Arc::new(store))
    }
}

impl ReadStore for Arc<BinaryStore> {
    fn query_path_info<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            // TODO: handle disabled
            let uri = format!("{}{}.narinfo", self.base_uri, path.hash_part());

            let response = self.client.get(&uri).send().await?;

            if response.status() == reqwest::StatusCode::NOT_FOUND
                || response.status() == reqwest::StatusCode::FORBIDDEN
            {
                warn!(
                    "got non nice status coder for '{}': {}",
                    uri,
                    response.status()
                );
                return Err(StoreError::NotInStore {
                    path: self.print_store_path(path),
                });
            }

            let body = response.text().await?;

            ValidPathInfo::parse_str(&body, self)
        }))
    }

    fn is_valid_path<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<bool, StoreError>> {
        unimplemented!()
    }

    fn box_clone_read(&self) -> Box<dyn ReadStore> {
        Box::new(self.clone())
    }
}

impl Store for Arc<BinaryStore> {
    fn get_state_dir<'a>(&'a self) -> Result<String, StoreError> {
        unimplemented!()
    }

    fn get_store_dir<'a>(&'a self) -> Result<String, StoreError> {
        Ok(self.store_dir.to_string())
    }

    fn get_uri<'a>(&'a self) -> String {
        self.base_uri.clone()
    }

    fn priority<'a>(&'a self) -> u64 {
        self.priority
    }

    fn capability<'a>(&'a self) -> super::StoreCap {
        super::StoreCap::Read // FIXME: write
    }

    fn box_clone(&self) -> Box<dyn Store> {
        Box::new(self.clone())
    }
}
