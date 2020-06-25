use std::convert::From;
use std::io;

use custom_error::custom_error;

custom_error! {
    pub StoreError
        Io{source: io::Error} = "IoError: {source}",
        StringToLong{len: usize} = "string is to long",
        ConnectionError{source: ConnectionError} = "ConnectionError: {source}",
        InvalidStoreUri{uri: String} = "InvalidStoreUri: {uri}",
        NotInStore{path: String} = "path \"{path}\" is not in the Nix store",
        UtilError{source: libutil::error::UtilError} = "UtilError: {source}",
        SqlError{source: rusqlite::Error} = "SQLError: {source}",
        OsError{ call: String, ret: i32 } = "Os Error: {call}: {ret}",
}

custom_error! {
    pub ConnectionError
        Io{source: io::Error} = "IoError: {source}",
}
