#![allow(unused_variables)]
#![allow(dead_code)]
// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

use std::sync::{Arc, Mutex};

use super::{ReadStore, Store, StoreError, StorePath, ValidPathInfo, WriteStore};

use std::collections::HashMap;

use log::*;

#[derive(Debug)]
pub struct File {
    pub content: Vec<u8>,
    pub executable: bool,
}

#[derive(Clone, Debug)]
pub struct MockStore {
    files: Arc<Mutex<HashMap<String, File>>>,
    symlinks: Arc<Mutex<HashMap<String, String>>>,
    dirs: Arc<Mutex<Vec<String>>>,
}

impl MockStore {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            symlinks: Arc::new(Mutex::new(HashMap::new())),
            dirs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn file_exists(&self, path: &str) -> bool {
        let files = self.files.lock().unwrap();
        files.get(path).is_some()
    }

    pub fn link_exists(&self, path: &str) -> bool {
        let symlinks = self.symlinks.lock().unwrap();
        symlinks.get(path).is_some()
    }

    pub fn dir_exists(&self, path: &str) -> bool {
        let dirs = self.dirs.lock().unwrap();
        dirs.contains(&path.to_owned())
    }

    pub fn file_as_string(&self, path: &str) -> String {
        let files = self.files.lock().unwrap();
        String::from_utf8_lossy(&files.get(path).unwrap().content).to_string()
    }

    pub fn symlinks_points_at(&self, path: &str) -> String {
        let symlinks = self.symlinks.lock().unwrap();
        symlinks.get(path).unwrap().clone()
    }

    pub fn is_file_executable(&self, path: &str) -> bool {
        let files = self.files.lock().unwrap();
        files.get(path).unwrap().executable
    }
}

impl WriteStore for Arc<MockStore> {
    fn write_file<'a>(
        &'a self,
        path: &'a str,
        data: &'a [u8],
        executable: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            info!("add file {} to mock store", path);
            let file = File {
                executable,
                content: data.to_owned(),
            };
            let mut files = self.files.lock().unwrap();
            files.insert(path.to_string(), file);
            Ok(())
        }))
    }

    fn make_symlink<'a>(
        &'a self,
        source: &str,
        target: &str,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        let mut symlinks = self.symlinks.lock().unwrap();
        symlinks.insert(source.to_string(), target.to_string());
        LocalFutureObj::new(Box::new(async { Ok(()) }))
    }
    fn make_directory<'a>(&'a self, path: &str) -> LocalFutureObj<'a, Result<(), StoreError>> {
        let path = path.to_owned();
        LocalFutureObj::new(Box::new(async move {
            info!("add directory {} to mock store", path);
            let mut dirs = self.dirs.lock().unwrap();
            dirs.push(path);
            Ok(())
        }))
    }

    fn delete_path<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!()
    }

    fn create_user<'a>(
        &'a self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!()
    }

    fn register_path<'a>(
        &'a self,
        info: ValidPathInfo,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        unimplemented!()
    }

    fn add_temp_root<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!()
    }

    fn add_text_to_store<'a>(
        &'a self,
        suffix: &'a str,
        data: &'a [u8],
        refs: &'a super::path::StorePaths,
        repair: bool,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        unimplemented!()
    }

    fn add_to_store<'a>(
        &'a self,
        path: ValidPathInfo,
        repair: bool,
        check_sigs: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        unimplemented!()
    }

    fn box_clone_write(&self) -> Box<dyn WriteStore> {
        Box::new(self.clone())
    }
}

impl ReadStore for Arc<MockStore> {
    fn query_path_info<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        unimplemented!()
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

impl Store for Arc<MockStore> {
    fn get_store_dir<'a>(&'a self) -> Result<String, StoreError> {
        Ok("/nix/store".to_string())
    }

    fn get_state_dir<'a>(&'a self) -> Result<String, StoreError> {
        unimplemented!("store: get_state_dir")
    }

    fn box_clone(&self) -> Box<dyn Store> {
        Box::new(self.clone())
    }
}
