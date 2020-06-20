use crate::error::StoreError;

use std::sync::{Arc, RwLock};

pub mod local_store;

pub trait Store {
    

    async fn create_user(&mut self, username: &str, uid: u32) -> Result<(), StoreError> { Ok(()) }
}

pub fn openStore(store_uri: &str, params: std::collections::HashMap<String, String>) -> Result<Box<dyn Store>, StoreError> {
    if store_uri == "auto" {
        let store = local_store::LocalStore::openStore("/nix/", params)?;
        return Ok(Box::new(store));
    }

    // FIXME: magic for other store bachends
    if !store_uri.starts_with("file://") {
        return Err(crate::error::StoreError::InvalidStoreUri{ uri: store_uri.to_string() });
    }

    let path = &store_uri["file://".len()..];
    let store = local_store::LocalStore::openStore(path, params)?;
    Ok(Box::new(store))

}