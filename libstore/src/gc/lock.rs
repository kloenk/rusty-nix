use std::os::unix::io::{AsRawFd, RawFd};

#[repr(i32)]
pub enum LockType {
    Read = libc::LOCK_SH,
    Write = libc::LOCK_EX,
    None = libc::LOCK_UN,
}

pub fn lock_file<T: AsRawFd>(fd: &T, lock_type: LockType, wait: bool) -> std::io::Result<bool> {
    lock_file_fd(fd.as_raw_fd(), lock_type, wait)
}

pub fn lock_file_fd(fd: RawFd, lock_type: LockType, wait: bool) -> std::io::Result<bool> {
    let mut lock_type = lock_type as i32;
    if !wait {
        lock_type = lock_type | libc::LOCK_NB;
    }

    if unsafe { libc::flock(fd, lock_type) } != 0 {
        let errno = nix::errno::errno();
        if !wait && errno == libc::EWOULDBLOCK {
            return Ok(false);
        }
        return Err(std::io::Error::from_raw_os_error(errno));
    }
    Ok(true)
}

#[cfg(test)]
mod test {
    #[test]
    fn write_block() {
        let file = std::fs::File::create("/tmp/nix-test-lock-file").unwrap();
        assert!(super::lock_file(&file, super::LockType::Write, true).unwrap());

        let file = std::fs::File::create("/tmp/nix-test-lock-file").unwrap();
        assert!(!super::lock_file(&file, super::LockType::Write, false).unwrap());
    }
}
