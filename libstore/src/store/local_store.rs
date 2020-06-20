
use crate::error::StoreError;
use log::trace;

pub struct LocalStore {
    baseDir: String,
    params: std::collections::HashMap<String, String>,
}

impl LocalStore {
    pub fn openStore(path: &str, params: std::collections::HashMap<String, String>) -> Result<Self, crate::error::StoreError> {
        // TODO: access checks?
        trace!("opening local store {}", path);
        std::fs::create_dir_all(path)?;
        Ok(Self {
            baseDir: path.to_string(),
            params,
        })
    }
}

impl crate::Store for LocalStore {
    fn create_user(&mut self, username: &str, uid: u32) -> std::future::Future<Output = Result<(), StoreError>> {
       let dirs = vec![
            format!("{}/profiles/per-user/{}", self.baseDir, username),
            format!("{}/gcroots/per-user/{}", self.baseDir, username),
       ];

       for dir in dirs {
           std::fs::create_dir_all(&dir)?;
           use std::os::unix::fs::PermissionsExt;
           let perms = std::fs::Permissions::from_mode(0o755);
           std::fs::set_permissions(&dir, perms)?;
           // TODO: chown
       }

       Ok(())
    }
}