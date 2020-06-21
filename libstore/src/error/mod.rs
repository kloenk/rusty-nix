use std::convert::From;
use std::io;

use custom_error::custom_error;

custom_error! {
    pub StoreError
        Io{source: io::Error} = "IoError: {source}",
        ConnectionError{source: ConnectionError} = "ConnectionError: {source}",
        InvalidStoreUri{uri: String} = "InvalidStoreUri: {uri}",
        NotInStore{path: String} = "path \"{path}\" is not in the Nix store",
        UtilError{source: libutil::error::UtilError} = "UtilError: {source}",
        SqlError{source: rusqlite::Error} = "SQLError: {source}",
}

custom_error! {
    pub ConnectionError
        Io{source: io::Error} = "IoError: {source}",
}
