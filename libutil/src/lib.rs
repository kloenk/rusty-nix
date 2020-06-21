use std::path::Path;

use crate::error::{Result, UtilError};

pub mod config;
pub mod error;

pub async fn canon_path<'a>(path: &'a str) -> Result<&'a Path> {
    if path == "" {
        return Err(UtilError::EmptyPath {});
    }

    if !path.starts_with('/') {
        return Err(UtilError::NotAbsolute {
            path: path.to_string(),
        });
    }

    // TODO: nix does some things here??

    Ok(Path::new(path))
}

pub async fn canon_path_resolve_symlink(path: &str) -> Result<&Path> {
    let mut follow_count: usize = 0;
    const MAX_FOLLOW: usize = 1024;
    unimplemented!()
}
