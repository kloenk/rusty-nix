#![allow(unused_variables)]
#![allow(dead_code)]
// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

use super::{Store, StoreError, ValidPathInfo};

use std::collections::HashMap;

use log::*;

pub struct File {
    pub content: Vec<u8>,
    pub executable: bool,
}

pub struct MockStore {
    files: HashMap<String, File>,
    symlinks: HashMap<String, String>,
    dirs: Vec<String>,
}

impl MockStore {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            symlinks: HashMap::new(),
            dirs: Vec::new(),
        }
    }

    pub fn file_exists(&self, path: &str) -> bool {
        self.files.get(path).is_some()
    }

    pub fn link_exists(&self, path: &str) -> bool {
        self.symlinks.get(path).is_some()
    }

    pub fn dir_exists(&self, path: &str) -> bool {
        self.dirs.contains(&path.to_owned())
    }

    pub fn file_as_string(&self, path: &str) -> String {
        String::from_utf8_lossy(&self.files.get(path).unwrap().content).to_string()
    }

    pub fn symlinks_points_at(&self, path: &str) -> String {
        self.symlinks.get(path).unwrap().clone()
    }

    pub fn is_file_executable(&self, path: &str) -> bool {
        self.files.get(path).unwrap().executable
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
        let path = path.to_owned();
        LocalFutureObj::new(Box::new(async move {
            info!("add file {} to mock store", path);
            let file = File {
                executable,
                content: data.to_owned(),
            };
            self.files.insert(path, file);
            Ok(())
        }))
    }

    fn make_directory<'a>(&'a mut self, path: &str) -> LocalFutureObj<'a, Result<(), StoreError>> {
        let path = path.to_owned();
        LocalFutureObj::new(Box::new(async move {
            info!("add directory {} to mock store", path);
            self.dirs.push(path);
            Ok(())
        }))
    }

    fn make_symlink<'a>(
        &'a mut self,
        source: &str,
        target: &str,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        self.symlinks.insert(source.to_string(), target.to_string());
        LocalFutureObj::new(Box::new(async { Ok(()) }))
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

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}
