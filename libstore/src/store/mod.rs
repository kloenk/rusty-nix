use crate::error::StoreError;

use std::sync::{Arc, RwLock};

// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

pub mod local_store;
pub mod protocol;

pub trait Store {
    fn create_user<'a>(
        &'a mut self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>>;
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
