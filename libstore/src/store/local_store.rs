use crate::error::StoreError;
use log::{debug, trace, warn};

// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

use std::sync::{Arc, RwLock};

pub struct LocalStore {
    base_dir: String,
    params: std::collections::HashMap<String, super::Param>,

    sqlite: Arc<RwLock<rusqlite::Connection>>,
}

impl LocalStore {
    pub async fn open_store(
        path: &str,
        params: std::collections::HashMap<String, super::Param>,
    ) -> Result<Self, crate::error::StoreError> {
        // TODO: access checks?
        trace!("opening local store {}", path);
        trace!("got params: {:?}", params);
        std::fs::create_dir_all(path)?;

        let sqlite = Arc::new(RwLock::new(rusqlite::Connection::open(&format!(
            "{}/var/nix/db/db.sqlite",
            path
        ))?));

        let store = Self {
            base_dir: path.to_string(),
            params,
            sqlite,
        };

        store.make_store_writable().await?;

        Ok(store)
    }

    #[cfg(target_os = "linux")]
    async fn make_store_writable(&self) -> Result<(), StoreError> {
        debug!("remounting store");
        if unsafe { libc::getuid() } != 0 {
            return Ok(());
        }

        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        let store_dir = std::ffi::CString::new(self.get_store_dir().as_str()).unwrap();
        if unsafe { libc::statvfs(store_dir.as_ptr(), stat.as_mut_ptr()) } != 0 {
            return Err(StoreError::SysError {
                msg: format!(
                    "getting info about the nix store mount point: {}",
                    self.get_store_dir()
                ),
            });
        }

        if (unsafe { stat.assume_init().f_flag } & libc::ST_RDONLY) != 0 {
            if unsafe { libc::unshare(libc::CLONE_NEWNS) } == -1 {
                return Err(StoreError::SysError {
                    msg: String::from("setting up a private mount namespace"),
                });
            }

            if unsafe {
                libc::mount(
                    &0,
                    store_dir.as_ptr(),
                    std::ffi::CString::new("none").unwrap().as_ptr(),
                    libc::MS_REMOUNT | libc::MS_BIND,
                    std::ptr::null::<std::ffi::c_void>(),
                )
            } == -1
            {
                return Err(StoreError::SysError {
                    msg: format!("remounting store writable: {}", self.get_store_dir()),
                });
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    async fn make_store_writable<'a>(&'a mut self) -> Result<(), StoreError> {
        Ok(())
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

            let path = path.canonicalize();
            if let Err(v) = &path {
                if v.kind() == std::io::ErrorKind::NotFound {
                    trace!("cannot canon path");
                    return Ok(false);
                }
            }
            let path = path?;
            if path.parent().unwrap() != std::path::Path::new(&self.get_store_dir().await?) {
                return Err(StoreError::NotInStore {
                    path: path.to_string_lossy().to_string(),
                });
            }
            let sqlite = self.sqlite.write().unwrap();
            let mut stm = sqlite.prepare("SELECT id FROM ValidPaths WHERE path = (?);")?;

            let data = stm
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

            let hash_part = get_hash_part(&path);
            // TODO: implement lru cache

            // TODO: check for disk cache
            let sqlite = self.sqlite.write().unwrap();
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
                let nar_size: Option<u64> = row.get::<usize, isize>(4).map(|v| v as u64).ok();
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
                    nar_size: nar_size,
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

    fn add_temp_root<'a>(
        &'a mut self,
        path: &std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            warn!("add_temp_root not yet implemented for LocalStore");
            Ok(())
        }))
    }

    fn delete_path<'a>(
        &'a mut self,
        path: &std::path::PathBuf,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            warn!("delete_path not yet implemented");
            Ok(())
        }))
    }

    fn add_to_store<'a>(
        &'a mut self,
        path: super::ValidPathInfo,
        repair: bool,
        check_sigs: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if let super::Hash::None = path.nar_hash {
                return Err(StoreError::MissingHash {
                    path: path.path.display().to_string(),
                });
            }
            // TODO: return err if sig is missing

            self.add_temp_root(&path.path).await?;

            if repair || !self.is_valid_path(&path.path).await? {
                self.delete_path(&path.path);

                /*if path.ca.is_some() {
                    let ca = path.ca.unwrap();
                    if !ca.starts_with("text:") && path.references.len() == 0 || path.references.len() == 0
                }*/

                //if path.ca != "" && !(path.ca.starts_with("text:") && path.references.len() == 0) || path.references.len() == 0) TODO: what???
                // requireFeature("ca-references")

                //                self.registerValidPath(path).await?;

                /* std::unique_ptr<AbstractHashSink> hashSink;
                if (info.ca == "" || !info.references.count(info.path))
                    hashSink = std::make_unique<HashSink>(htSHA256);
                else
                    hashSink = std::make_unique<HashModuloSink>(htSHA256, std::string(info.path.hashPart()));

                LambdaSource wrapperSource([&](unsigned char * data, size_t len) -> size_t {
                    size_t n = source.read(data, len);
                    (*hashSink)(data, n);
                    return n;
                });

                restorePath(realPath, wrapperSource);

                auto hashResult = hashSink->finish();

                if (hashResult.first != info.narHash)
                    throw Error("hash mismatch importing path '%s';\n  wanted: %s\n  got:    %s",
                        printStorePath(info.path), info.narHash.to_string(Base32, true), hashResult.first.to_string(Base32, true));

                if (hashResult.second != info.narSize)
                    throw Error("size mismatch importing path '%s';\n  wanted: %s\n  got:   %s",
                        printStorePath(info.path), info.narSize, hashResult.second);

                autoGC();

                canonicalisePathMetaData(realPath, -1);

                optimisePath(realPath); // FIXME: combine with hashPath()

                registerValidPath(info); */
            }

            // outputLock.setDeletion

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
