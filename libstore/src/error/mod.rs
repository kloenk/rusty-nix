use std::convert::From;
use std::io;

use custom_error::custom_error;

custom_error! {
    pub CommandError
        Io{source: io::Error} = "IoError: {source}",
}

impl CommandError {
    pub fn get_code(&self) -> i32 {
        match self {
            CommandError::Io { source: _ } => 2,
        }
    }
}

pub type CommandResult<T> = std::result::Result<T, CommandError>;
