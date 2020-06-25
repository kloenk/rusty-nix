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
                let dir = std::ffi::CString::new(dir.as_str()).unwrap(); // TODO: error handling
                let chown = unsafe { libc::chown(dir.as_ptr(), uid, libc::getgid()) };
                if chown != 0 {
                    return Err(StoreError::OsError {
                        call: String::from("chown"),
                        ret: chown,
                    });
                }
            }

            Ok(())
        }))
    }

    fn is_valid_path<'a>(
        &'a mut self,
        path: &'a std::path::Path,
    ) -> LocalFutureObj<'a, Result<bool, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if path.to_str().unwrap() == "" {
                return Err(StoreError::NotInStore {
                    path: path.to_string_lossy().to_string(),
                });
            }

            let path = path.canonicalize()?;
            if path.parent().unwrap() != std::path::Path::new(&self.get_store_dir().await?) {
                return Err(StoreError::NotInStore {
                    path: path.to_string_lossy().to_string(),
                });
            }

            let mut sqlite = self.sqlite.write().unwrap();
            let mut stm = sqlite.prepare("SELECT id FROM ValidPaths WHERE path = (?);")?;

            let mut data = stm
                .query_row(&[&path.to_str()], |row| Ok(true))
                .unwrap_or(false);

            Ok(data)
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
                let id: u64 = row.get::<usize, isize>(0)? as u64;
                let nar_hash: crate::store::Hash = row
                    .get::<usize, String>(1)
                    .map(|v| crate::store::Hash::from(v.as_str()))?;
                let registration_time: chrono::NaiveDateTime = row
                    .get::<usize, isize>(2)
                    .map(|v| chrono::NaiveDateTime::from_timestamp(v as i64, 0))?;
                let deriver: Option<std::path::PathBuf> = row
                    .get::<usize, String>(3)
                    .map(|v| std::path::PathBuf::from(v))
                    .ok();
                let narSize: Option<u64> = row.get::<usize, isize>(4).map(|v| v as u64).ok();
                let ultimate: bool = row.get::<usize, isize>(5).unwrap_or(0) != 0;
                let sigs: Vec<String> = row
                    .get::<usize, String>(6)
                    .map(|v| v.split(' ').map(|v| v.to_string()).collect())
                    .unwrap_or(Vec::new());
                let ca: Option<String> = row.get::<usize, String>(7).ok();
                Ok(crate::store::ValidPathInfo {
                    path: std::path::PathBuf::from(&path),
                    deriver,
                    nar_hash,
                    references: Vec::new(), // TODO: referecnes foo
                    registration_time,
                    narSize,
                    id,
                    ultimate,
                    sigs,
                    ca,
                }) // TODO: return valid Path Info
            })?;

            //let data = data.next().ok_or_else(|| -> Result<Valid> { Err(StoreError::NotInStore{ path: path.display().to_string(), } ) } )).unwrap();
            //let data = data?;
            let mut data = data.next().ok_or(StoreError::NotInStore {
                path: path.display().to_string(),
            })??;

            let mut ref_stm = sqlite.prepare("SELECT reference FROM Refs WHERE referrer = (?);")?;
            let refs = ref_stm.query_map(&[data.id as isize], |row| {
                let reffercens = row.get::<usize, isize>(0)? as usize;
                Ok(reffercens)
            })?;

            let mut stm = sqlite.prepare("SELECT path FROM ValidPaths WHERE id = (?);")?;

            for v in refs {
                let v = v? as isize;
                if v as u64 == data.id {
                    continue;
                } // has itsself as refferencse
                let row = stm.query_row(&[v], |row| {
                    let path = std::path::PathBuf::from(row.get::<usize, String>(0)?);
                    Ok(path)
                })?;
                data.references.push(row);
            }

            trace!("{:?}", data);

            Ok(data) // TODO: no unwrap
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
