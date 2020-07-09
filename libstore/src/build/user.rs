use libc::{gid_t, uid_t};

use log::*;

use crate::error::BuildError;

#[derive(Debug)]
pub struct UserLock {
    user: String,
    uid: uid_t,
    gid: gid_t,
    supplementary_gids: Vec<gid_t>,

    file: std::fs::File,
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

                let s_gids = Self::get_supplementary_gids(&user)?;

                return Ok(Self {
                    gid: group.gid.as_raw(),
                    supplementary_gids: s_gids,
                    uid: user.uid.as_raw(),
                    user: user.name,
                    file,
                });
            }
        }

        Err(BuildError::NoFreeUsers {})
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

    pub fn get_uid(&self) -> uid_t {
        self.uid
    }
}

impl Drop for UserLock {
    fn drop(&mut self) {
        kill_user(self.uid).unwrap();
    }
}

/// # Panics
/// This function pannics if the uid is 0
pub fn kill_user(uid: libc::uid_t) -> Result<(), BuildError> {
    debug!("killing all processes running under uid {}", uid);

    assert_ne!(uid, 0);

    /* The system call kill(-1, sig) sends the signal `sig' to all
    users to which the current process can send signals.  So we
    fork a process, switch to uid, and send a mass kill. */

    let killer: std::thread::JoinHandle<Result<(), BuildError>> = std::thread::spawn(move || {
        if unsafe { libc::setuid(uid) } == -1 {
            return Err(BuildError::SysError {
                msg: "setting uid".to_string(),
            });
        }

        loop {
            if cfg!(target_os = "darwin") {
                unimplemented!("syscall call?? https://github.com/NixOS/nix/blob/4d5169bdd507b12d8fe0a1cab89b5d81a43e6de5/src/libutil/util.cc#L923");
            }

            if unsafe { libc::kill(-1, libc::SIGKILL) } == 0 {
                break;
            };

            let errno = nix::errno::errno();
            if errno == libc::ESRCH {
                break;
            } // no more processes
            if errno != libc::EINTR {
                return Err(BuildError::SysError {
                    msg: format!("cannot kill processes for uid '{}'", uid),
                }); // TODO: should this be an IO error?
            }
        }

        Ok(())
    });

    killer.join().unwrap()?;
    Ok(())
}

impl PartialEq for UserLock {
    fn eq(&self, other: &Self) -> bool {
        return self.uid == other.uid;
    }
}

impl Eq for UserLock {}

#[cfg(test)]
mod test {
    #[test]
    #[should_panic]
    fn kill_user_root() {
        super::kill_user(0).unwrap();
    }

    #[test]
    #[ignore]
    fn kill_user() {
        // setup config
        let mut config = libutil::config::NixConfig::default();
        config.build_users_group = "nixbld".to_string(); // this test could fail on a non std nix setup
        let mut cfg = crate::CONFIG.write().unwrap();
        *cfg = config;

        drop(cfg);
        let user = super::UserLock::find_free_user().unwrap();
        let uid = user.get_uid();
        let pid = std::thread::spawn(move || {
            if unsafe { libc::setuid(uid) } != 0 {
                panic!()
            }

            let process = std::process::Command::new("sleep")
                .arg("20h")
                .spawn()
                .unwrap();
            println!("The pid of 'sleep' is {}", process.id());
            process.id()
        })
        .join()
        .unwrap();

        let cmdline_1 = std::fs::read_to_string(format!("/proc/{}/cmdline", pid)).unwrap();
        drop(user);
        let cmdline_2 =
            std::fs::read_to_string(format!("/proc/{}/cmdline", pid)).unwrap_or("".to_string());
        assert_ne!(cmdline_1, cmdline_2);
    }

    #[test]
    #[ignore]
    /// This tests needs a default nix setup with the default buildgroup (nixbld)
    /// also this tests needs root permissions
    fn lock_2_users() {
        // Populate config
        let mut config = libutil::config::NixConfig::default();
        config.build_users_group = "nixbld".to_string(); // this test could fail on a non std nix setup
        let mut cfg = crate::CONFIG.write().unwrap();
        *cfg = config;
        drop(cfg);

        // this could fail if nix is building something big
        let user_1 = super::UserLock::find_free_user().unwrap();
        let user_2 = super::UserLock::find_free_user().unwrap();

        assert_ne!(user_1, user_2);
    }
}
