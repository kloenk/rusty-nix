use std::io;

use custom_error::custom_error;

custom_error! {
    pub UtilError
        Io{source: io::Error} = "IoError: {source}",
        EmptyPath{} = "Path is empty",
        NotAbsolute{path: String} = "not an absolute path: {path}",
}

pub type Result<T> = std::result::Result<T, UtilError>;
