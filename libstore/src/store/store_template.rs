#![allow(unused_variables)]
#![allow(dead_code)]
// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

use super::{Store, StoreError, ValidPathInfo};

pub struct MockStore {}

impl MockStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl Store for MockStore {
    fn get_store_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>> {
        LocalFutureObj::new(Box::new(async move { Ok("/nix/store".to_string()) }))
    }

    fn get_state_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>> {
        unimplemented!("store: get_state_dir")
    }

    fn create_user<'a>(
        &'a mut self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!("store: create_user")
    }

    fn query_path_info<'a>(
        &'a mut self,
        path: std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        unimplemented!("store: query_path_info")
    }

    fn is_valid_path<'a>(
        &'a mut self,
        path: &'a std::path::Path,
    ) -> LocalFutureObj<'a, Result<bool, StoreError>> {
        unimplemented!("store: is_valid_path")
    }

    fn register_path<'a>(
        &'a mut self,
        info: ValidPathInfo,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        unimplemented!("store: register_path")
    }

    fn delete_path<'a>(
        &'a mut self,
        path: &std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!("store: delete_path")
    }

    fn add_to_store<'a>(
        &'a mut self,
        path: ValidPathInfo,
        repair: bool,
        check_sigs: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!("store: add_to_store")
    }

    fn write_file<'a>(
        &'a mut self,
        path: &str,
        data: &'a [u8],
        executable: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!("store: write_file")
    }

    fn make_directory<'a>(&'a mut self, path: &str) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!("store: make_directory")
    }

    fn add_text_to_store<'a>(
        &'a mut self,
        suffix: &'a str,
        data: &'a [u8],
        refs: &'a Vec<String>,
        repair: bool,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        unimplemented!("store: add_text_to_store")
    }

    fn add_temp_root<'a>(&'a mut self, path: &str) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!("store: add_temp_root")
    }
}
