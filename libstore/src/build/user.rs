use libc::{gid_t, uid_t};

use log::*;

use crate::error::BuildError;

#[derive(Debug)]
pub struct UserLock {
    user: String,
    uid: uid_t,
    gid: gid_t,
    supplementary_gids: Vec<gid_t>,
}

impl UserLock {
    pub fn find_free_user() -> Result<Self, BuildError> {
        let config = crate::CONFIG.read().unwrap();
        let build_user_group = config.build_users_group.clone();
        let state_dir = config.nix_state_dir.clone();
        drop(config);

        if build_user_group.is_empty() {
            return Err(BuildError::NoBuildUsers {});
        }

        std::fs::create_dir_all(format!("{}/userpool", state_dir))?;

        //let group = users::get_group_by_name(&build_user_group); // cannot be used here, does not return users in the group
        //let group = get_group_by_name(&build_user_group)?;
        let group = nix::unistd::Group::from_name(&build_user_group)
            .unwrap()
            .unwrap();
        if group.mem.is_empty() {
            return Err(BuildError::NoBuildUsers {});
        }

        for v in &group.mem {
            debug!("trying user '{}'", v);

            let user = nix::unistd::User::from_name(&v).unwrap();
            if user.is_none() {
                return Err(BuildError::UserNotExisting {
                    user: v.to_string(),
                });
            }
            let user = user.unwrap();

            let fn_user_lock = format!("{}/userpool/{}", state_dir, user.uid);
            let file = std::fs::File::create(fn_user_lock)?;
            if crate::gc::lock::lock_file(&file, crate::gc::lock::LockType::Write, false)? {
                if user.uid.as_raw() == unsafe { libc::getuid() }
                    || user.uid.as_raw() == unsafe { libc::geteuid() }
                {
                    return Err(BuildError::UserInGroup {
                        group: build_user_group,
                    });
                }

                let s_gids = get_supplementary_gids(&user)?;

                return Ok(Self {
                    gid: group.gid.as_raw(),
                    supplementary_gids: s_gids,
                    uid: user.uid.as_raw(),
                    user: user.name,
                });
            }
        }

        unimplemented!();

        /*Ok( Self {
            gid: group.gr_gid,
        })*/
    }
}

#[cfg(target_os = "linux")]
fn get_supplementary_gids(user: &nix::unistd::User) -> std::io::Result<Vec<libc::gid_t>> {
    let user_name = std::ffi::CString::new(user.name.as_bytes()).unwrap();
    let list: Vec<nix::unistd::Gid> =
        nix::unistd::getgrouplist(&user_name, user.gid).map_err(|v| match v {
            nix::Error::Sys(v) => std::io::Error::from_raw_os_error(v as i32),
            _ => unimplemented!(),
        })?;

    Ok(list.iter().map(|v| v.as_raw()).collect())
}

#[cfg(not(target_os = "linux"))]
fn get_supplementary_gids(_user: &nix::unistd::User) -> std::io::Result<Vec<libc::gid_t>> {
    Vec::new()
}
