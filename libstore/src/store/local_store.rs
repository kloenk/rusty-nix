use crate::error::StoreError;
use log::trace;

// for async trait
use futures::future::LocalFutureObj;
use std::boxed::Box;

pub struct LocalStore {
    base_dir: String,
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
            base_dir: path.to_string(),
            params,
        })
    }

    pub fn get_state_dir(&self) -> String {
        format!("{}/var/nix/", self.base_dir)
    }
}

impl crate::Store for LocalStore {
    fn create_user<'a>(
        &'a mut self,
        username: String,
        uid: u32,
    ) -> LocalFutureObj<'a, Result<(), StoreError>> {
        let state_dir = self.get_state_dir();
        LocalFutureObj::new(Box::new(async move {
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
}
