use std::io;

use custom_error::custom_error;

custom_error! {
    pub CommandError
        Io{source: io::Error} = "IoError: {source}",
        UtilParse{source: libutil::config::error::Error} = "parsing error: {source}",
        Tokio{source: tokio::task::JoinError} = "tokio error: {source}",
}

impl CommandError {
    pub fn get_code(&self) -> i32 {
        match self {
            CommandError::Io { .. } => 2, // TODO: get from io::Error (source)
            CommandError::UtilParse { .. } => 3,
            CommandError::Tokio { .. } => 4,
        }
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
