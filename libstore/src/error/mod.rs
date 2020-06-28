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
        MissingHash{path: String} = "{} lacks valid signature",
        OsError{ call: String, ret: i32 } = "Os Error: {call}: {ret}",
        SysError{ msg: String } = "SysError: {msg}",
        InvalidKey{ key: String } = "The key {key} is invalid",
        RingError{ source: ring::error::Unspecified } = "error in crypto: {source}",
        NoFingerprint{ path: String } = "cannot calculate fingerprint of path {path} because its size/hash is not known",
        HashDecodeError { source: data_encoding::DecodeError } = "cannot decode: {source}",
        //HashDecodePartialError { source: data_encoding::DecodePartial } = "cannot decode {source}", // TODO: to_string missing on DecodePartial
        HashDecodePartialError { error: String } = "cannont decode {error}",
}

custom_error! {
    pub ConnectionError
        Io{source: io::Error} = "IoError: {source}",
}
