use super::ValidPathInfo;
use crate::error::StoreError;
use crate::unimplemented;
use log::*;

// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

use super::path::StorePathWithOutputs;
use super::{BuildStore, ReadStore, Store, StorePath, WriteStore};

use std::sync::{Arc, RwLock};

#[allow(dead_code)]
#[derive(Clone, Debug)]
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
    async fn make_store_writable<'a>(&'a self) -> Result<(), StoreError> {
        Ok(())
    }

    pub fn get_state_dir(&self) -> String {
        format!("{}var/nix/", self.base_dir)
    }

    pub fn get_store_dir(&self) -> String {
        format!("{}store", self.base_dir)
    }
}

impl BuildStore for LocalStore {
    fn build_paths<'a>(
        &'a self,
        drvs: Vec<StorePathWithOutputs>,
        mode: u8,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            info!("building pathes: {:?}", drvs);

            let worker = crate::build::worker::Worker::new();

            self.prime_cache(&drvs).await?;

            unimplemented!()
        }))
    }

    fn query_missing<'a>(
        &'a self,
        paths: &'a Vec<StorePathWithOutputs>,
    ) -> LocalFutureObj<'a, Result<super::MissingInfo, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            warn!("unimplemented: LocalStore::query_missing");
            let libnfc = StorePath::new("05dxizl4i8w5k32x2kg3cxnim56cgvyy-libnfc-1.7.1.drv")?;
            Ok(super::MissingInfo {
                download_size: 0,
                nar_size: 0,
                unknown: Vec::new(),
                will_build: vec![libnfc],
                will_substitute: Vec::new(),
            })
        }))
    }
}

impl WriteStore for LocalStore {
    fn write_file<'a>(
        &'a self,
        path: &'a str,
        data: &'a [u8],
        executable: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            let mut file = tokio::fs::File::create(path).await?;

            use std::os::unix::fs::PermissionsExt;
            let perms = if executable { 0o555 } else { 0o444 };
            let perms = std::fs::Permissions::from_mode(perms);
            file.set_permissions(perms).await?;

            use tokio::io::AsyncWriteExt;
            file.write_all(data).await?;
            Ok(())
        }))
    }

    fn make_directory<'a>(&'a self, path: &str) -> LocalFutureObj<'a, Result<(), StoreError>> {
        let path = path.to_owned();
        LocalFutureObj::new(Box::new(async move {
            tokio::fs::create_dir_all(path).await?;
            Ok(())
        }))
    }

    fn make_symlink<'a>(
        &'a self,
        source: &'a str,
        target: &'a str,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            unimplemented!();
        }))
    }

    fn add_text_to_store<'a>(
        &'a self,
        suffix: &'a str,
        data: &'a [u8],
        refs: &'a Vec<String>,
        repair: bool,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            let hash = ring::digest::digest(&ring::digest::SHA256, data);
            let hash = super::Hash::from_sha256_vec(hash.as_ref())?;

            let dest_path = self.make_text_path(suffix, &hash, refs).await?;
            trace!("will write texte to {}", dest_path);

            self.add_temp_root(&dest_path).await?;

            if repair || !self.is_valid_path(&dest_path).await? {
                // TODO: make realpath?

                self.delete_path(&dest_path).await;
                let rm = tokio::fs::remove_file(&self.print_store_path(&dest_path)).await; // magic like moving to /nix/store/.thrash
                trace!("rm: {:?}", rm);

                //self.autoGC()

                /*let mut file = tokio::fs::File::create(&dest_path).await?;
                use tokio::io::AsyncWriteExt;
                file.write_all(data).await?;

                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o444);
                file.set_permissions(perms).await?;*/

                /*file.sync_all().await?; // TODO: put behind settings*/
                self.write_file(&self.print_store_path(&dest_path), data, false)
                    .await?;

                // dumpString(data)
                let nar = crate::archive::dump_data(&data);
                let hash = ring::digest::digest(&ring::digest::SHA256, &nar);
                let hash = super::Hash::from_sha256_vec(hash.as_ref())?;

                let mut info = ValidPathInfo::now(dest_path, hash, nar.len() as u64)?;
                // TODO: references, ca
                let info = self.register_path(info).await?;
                return Ok(info);
            }
            self.query_path_info(&dest_path).await
        }))
    }

    fn add_to_store<'a>(
        &'a self,
        path: super::ValidPathInfo,
        repair: bool,
        check_sigs: bool,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if let super::Hash::None = path.nar_hash {
                return Err(StoreError::MissingHash {
                    path: self.print_store_path(&path.path),
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

    fn delete_path<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        let path = self.print_store_path(path);
        LocalFutureObj::new(Box::new(async move {
            warn!("delete_path not yet implemented for : {}", &path);
            //unimplemented!("delete path"); // TODO: make less ugly

            #[allow(unused_must_use)]
            std::fs::remove_dir_all(&path);

            let sqlite = self.sqlite.write().unwrap();
            sqlite.execute("DELETE FROM ValidPaths WHERE path = (?);", &[&path])?;
            /*let mut stm = sqlite.prepare("DELETE FROM ValidPaths WHERE path = (?);")?;

            let data = stm.query_map(&[&path], |row| {
                warn!("foobar");
                Ok(())
            }).unwrap();*/

            Ok(())
        }))
    }

    fn register_path<'a>(
        &'a self,
        info: ValidPathInfo,
    ) -> LocalFutureObj<'a, Result<ValidPathInfo, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            trace!("will register path {:?}", info);
            // let path = info.path.canonicalize()?;
            /*if path.parent().unwrap() != std::path::Path::new(&self.get_store_dir()) {
                return Err(StoreError::NotInStore {
                    path: path.to_string_lossy().to_string(),
                });
            }*/ // TODO: should fail while parsing to an StorePath

            let sqlite = self.sqlite.write().unwrap();
            //let mut stm = sqlite.prepare("INSERT INTO ValidPaths (path, hash, registrationTime, deriver, narSize, ultimate, sigs, ca) values (?, ?, ?, ?, ?, ?, ?, ?);")?; // TODO: prepare those in a state object or so

            //let data = stm.execute(&path.display().to_string(), &info.nar_hash.to_sql_string(), &info.registration_time.timestamp().to_string(), &deriver, &nar_size, ]);
            /*let data = stm.execute(
                rusqlite::params![
                    path.display().to_string(),
                    info.nar_hash.to_sql_string(),
                    info.registration_time.timestamp(),
                    deriver,
                    nar_size,
                    info.ultimate,
                    "", // TODO: sigs
                    "", // TODO: CA
                ]
            )?;*/

            //sqlite.execute("INSERT INTO ValidPaths (path, hash, registrationTime, registrationTime) values (?path", )
            /*let mut params = rusqlite::named_params! {
                ":path": path.display().to_string(),
                ":hash": info.nar_hash.to_sql_string(),
                ":registrationTime": info.registration_time.timestamp().to_string(),

            };*/
            let path_str = self.print_store_path(&info.path);
            let hash = info.nar_hash.to_sql_string();
            let reg_time = info.registration_time.timestamp();
            let mut deriver = String::new();
            let mut nar_size = 0;
            let mut vec: Vec<(&str, &dyn rusqlite::ToSql)> = vec![
                (":path", &path_str),
                (":hash", &hash),
                (":registrationTime", &reg_time),
            ];
            if let Some(v) = &info.deriver {
                deriver = self.print_store_path(v);
                vec.push((":deriver", &deriver));
            }
            if let Some(nar) = info.nar_size {
                nar_size = nar as i64; //  u64 is not supported
                vec.push((":narSize", &nar_size));
            }
            if info.ultimate {
                vec.push((":ultimate", &1));
            }
            // TODO: sigs, ca
            let data = sqlite.execute_named("INSERT INTO ValidPaths (path, hash, registrationTime, deriver, narSize, ultimate, sigs, ca) values (:path, :hash, :registrationTime, :deriver, :narSize, :ultimate, :sigs, :ca);", &vec)?; // TODO: prepare those in a state object or so

            trace!("data: {:?}", data);

            let info = sqlite.query_row(
                "SELECT id FROM ValidPaths WHERE path = (?)",
                &[&path_str],
                move |row| {
                    let mut info = info.clone();
                    let id = row.get::<usize, isize>(0)?;
                    info.id = id as u64;
                    Ok(info)
                },
            )?;
            // TODO: references
            if info.references.len() != 0 {
                unimplemented!()
            }

            Ok(info)
        }))
    }

    fn add_temp_root<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            warn!("add temp root not yet implemented for '{}'", path);
            Ok(())
        }))
    }

    fn create_user<'a>(
        &'a self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        //let state_dir = self.get_state_dir();
        LocalFutureObj::new(Box::new(async move {
            let state_dir = self.get_state_dir();
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

    fn box_clone_write(&self) -> Box<dyn WriteStore> {
        Box::new(self.clone())
    }
}

impl ReadStore for LocalStore {
    fn query_path_info<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<crate::store::ValidPathInfo, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if path == "" {
                return Err(StoreError::NotInStore {
                    path: self.print_store_path(&path),
                });
            }
            /*let path = std::path::Path::new(path).canonicalize()?;
            if path.parent().unwrap() != std::path::Path::new(&self.get_store_dir()) {
                return Err(StoreError::NotInStore {
                    path: path.display().to_string(),
                });
            }*/

            let hash_part = path.hash_part();
            // TODO: implement lru cache

            // TODO: check for disk cache
            let sqlite = self.sqlite.write().unwrap();
            let mut stm = sqlite.prepare("SELECT id, hash, registrationTime, deriver, narSize, ultimate, sigs, ca FROM ValidPaths WHERE path = (?);")?;

            trace!("queriying for {} in sqlite", path);

            let mut data = stm.query_map(&[&self.print_store_path(&path)], |row| {
                let id: u64 = row.get::<usize, isize>(0)? as u64;
                let nar_hash: crate::store::Hash = row
                    .get::<usize, String>(1)
                    .map(|v| crate::store::Hash::from(v.as_str()))?;
                let registration_time: chrono::NaiveDateTime = row
                    .get::<usize, isize>(2)
                    .map(|v| chrono::NaiveDateTime::from_timestamp(v as i64, 0))?;
                let deriver: Option<StorePath> = row
                    .get::<usize, String>(3)
                    .map(|v| self.parse_store_path(&v).unwrap())
                    .ok();
                let nar_size: Option<u64> = row.get::<usize, isize>(4).map(|v| v as u64).ok();
                let ultimate: bool = row.get::<usize, isize>(5).unwrap_or(0) != 0;
                let sigs: Vec<String> = row
                    .get::<usize, String>(6)
                    .map(|v| v.split(' ').map(|v| v.to_string()).collect())
                    .unwrap_or(Vec::new());
                let ca: Option<String> = row.get::<usize, String>(7).ok();
                Ok(crate::store::ValidPathInfo {
                    path: path.clone(),
                    deriver,
                    nar_hash,
                    references: Vec::new(), // TODO: referecnes foo
                    registration_time,
                    nar_size,
                    id,
                    ultimate,
                    sigs,
                    ca,
                }) // TODO: return valid Path Info
            })?;

            //let data = data.next().ok_or_else(|| -> Result<Valid> { Err(StoreError::NotInStore{ path: path.display().to_string(), } ) } )).unwrap();
            //let data = data?;
            let mut data = data.next().ok_or(StoreError::NotInStore {
                path: path.to_string(),
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
                    let path = self
                        .parse_store_path(&row.get::<usize, String>(0)?)
                        .unwrap();
                    Ok(path)
                })?;
                data.references.push(row);
            }

            trace!("{:?}", data);

            Ok(data) // TODO: no unwrap
        }))
    }

    fn is_valid_path<'a>(
        &'a self,
        path: &'a StorePath,
    ) -> LocalFutureObj<'a, Result<bool, StoreError>> {
        LocalFutureObj::new(Box::new(async move {
            if path == "" {
                return Err(StoreError::NotInStore {
                    path: self.print_store_path(path),
                });
            }

            /*let path = path.canonicalize();
            if let Err(v) = &path {
                if v.kind() == std::io::ErrorKind::NotFound {
                    trace!("cannot canon path");
                    return Ok(false);
                }
            }
            let path = path?;
            if path.parent().unwrap() != std::path::Path::new(&self.get_store_dir()) {
                return Err(StoreError::NotInStore {
                    path: path.to_string_lossy().to_string(),
                });
            }*/
            let path = self.print_store_path(path);
            let sqlite = self.sqlite.write().unwrap();
            let mut stm = sqlite.prepare("SELECT id FROM ValidPaths WHERE path = (?);")?;

            let data = stm.query_row(&[&path], |row| Ok(true)).unwrap_or(false);

            Ok(data)
        }))
    }

    fn box_clone_read(&self) -> Box<dyn ReadStore> {
        Box::new(self.clone())
    }
}

impl Store for LocalStore {
    fn get_state_dir<'a>(&'a self) -> Result<String, StoreError> {
        Ok(format!("{}/var/nix/", self.base_dir))
    }

    fn get_store_dir<'a>(&'a self) -> Result<String, StoreError> {
        Ok(format!("{}store", self.base_dir))
    }

    fn box_clone(&self) -> Box<dyn Store> {
        Box::new(self.clone())
    }
}

// FIXME
fn get_hash_part(path: &std::path::PathBuf) -> String {
    let filename = path.file_name().unwrap();
    let filename = filename.to_string_lossy();

    let pos = filename.find('-').unwrap();
    String::from(&filename[0..pos])
}
