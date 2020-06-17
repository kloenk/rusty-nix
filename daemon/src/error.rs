use std::io;

use custom_error::custom_error;

custom_error! {
    pub CommandError
        Io{source: io::Error} = "IoError: {source}",
}

impl CommandError {
    pub fn get_code(&self) -> i32 {
        match self {
            CommandError::Io{ .. } => 2, // TODO: get from io::Error (source)
        }
    }
}

pub type CommandResult<T> = Result<T, CommandError>;