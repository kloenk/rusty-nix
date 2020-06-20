use crate::error::StoreError;
use log::trace;

// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

pub struct LocalStore {
    baseDir: String,
    stateDir: String,
    params: std::collections::HashMap<String, String>,
}

impl LocalStore {
    pub fn openStore(
        path: &str,
        params: std::collections::HashMap<String, String>,
    ) -> Result<Self, crate::error::StoreError> {
        // TODO: access checks?
        trace!("opening local store {}", path);
        std::fs::create_dir_all(path)?;
        Ok(Self {
            baseDir: path.to_string(),
            stateDir: format!("{}/var/nix", path),
            params,
        })
    }
}

impl crate::Store for LocalStore {
    fn create_user<'a>(
        &'a mut self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        let stateDir = self.stateDir.clone();
        LocalFutureObj::new(Box::new(async move {
            let dirs = vec![
                format!("{}/profiles/per-user/{}", &stateDir, username),
                format!("{}/gcroots/per-user/{}", &stateDir, username),
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
}
