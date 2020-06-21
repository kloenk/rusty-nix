use crate::error::StoreError;
use log::trace;

// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

use std::sync::{Arc, RwLock};

pub struct LocalStore {
    base_dir: String,
    params: std::collections::HashMap<String, String>,

    sqlite: Arc<RwLock<rusqlite::Connection>>,
}

impl LocalStore {
    pub fn openStore(
        path: &str,
        params: std::collections::HashMap<String, String>,
    ) -> Result<Self, crate::error::StoreError> {
        // TODO: access checks?
        trace!("opening local store {}", path);
        std::fs::create_dir_all(path)?;

        let sqlite = Arc::new(RwLock::new(rusqlite::Connection::open(&format!(
            "{}/var/nix/db/db.sqlite",
            path
        ))?));

        Ok(Self {
            base_dir: path.to_string(),
            params,
            sqlite,
        })
    }

    pub fn get_state_dir(&self) -> String {
        format!("{}/var/nix/", self.base_dir)
    }

    pub fn get_store_dir(&self) -> String {
        format!("{}/store", self.base_dir)
    }
}

impl crate::Store for LocalStore {
    fn get_state_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            Ok(format!("{}/var/nix/", self.base_dir))
        }))
    }

    fn get_store_dir<'a>(&'a mut self) -> LocalFutureObj<'a, Result<String, StoreError>> {
        LocalFutureObj::new(Box::new(
            async move { Ok(format!("{}/store/", self.base_dir)) },
        ))
    }

    fn create_user<'a>(
        &'a mut self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        //let state_dir = self.get_state_dir();
        LocalFutureObj::new(Box::new(async move {
            let state_dir = self.get_state_dir().await?;
            let dirs = vec![
                format!("{}/profiles/per-user/{}", &state_dir, username),
                format!("{}/gcroots/per-user/{}", &state_dir, username),
            ];

            for dir in dirs {
                std::fs::create_dir_all(&dir)?;
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                std::fs::set_permissions(&dir, perms)?;
                // TODO: chown
            }

            Ok(())
        }))
    }

    fn query_path_info<'a>(
        &'a mut self,
        path: std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<crate::store::ValidPathInfo, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if path.to_str().unwrap() == "" {
                return Err(StoreError::NotInStore {
                    path: path.display().to_string(),
                });
            }
            let path = path.canonicalize()?;
            if path.parent().unwrap() != std::path::Path::new(&self.get_store_dir().await?) {
                return Err(StoreError::NotInStore {
                    path: path.display().to_string(),
                });
            }

            let hashPart = get_hash_part(&path);
            // TODO: implement lru cache

            // TODO: check for disk cache
            let mut sqlite = self.sqlite.write().unwrap();
            let mut stm = sqlite.prepare("SELECT id, hash, registrationTime, deriver, narSize, ultimate, sigs, ca FROM ValidPaths WHERE path = (?);")?;

            trace!("queriying for {} in sqlite", path.display());

            let mut data = stm.query_map(&[&path.to_str()], |row| {
                println!("sqlite map");
                let str: u64 = row.get::<usize, isize>(0)? as u64;
                println!("row: {:?}", str);
                Ok("foobar") // TODO: return valid Path Info
            })?;

            println!("data: {:?}", data.next().unwrap().unwrap()); // TODO: handle data

            unimplemented!()
        }))
    }
}

// FIXME
fn get_hash_part(path: &std::path::PathBuf) -> String {
    let filename = path.file_name().unwrap();
    let filename = filename.to_string_lossy();

    let pos = filename.find('-').unwrap();
    String::from(&filename[0..pos])
}
