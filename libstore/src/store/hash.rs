use super::{Store, StoreError};

use log::trace;

mod base32;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Hash {
    SHA256(Vec<u8>),
    Compressed(Vec<u8>),
    None,
}

impl Hash {
    pub fn from_sha256(v: &str) -> Result<Self, StoreError> {
        let mut buf: [u8; 32] = [0; 32];
        data_encoding::HEXLOWER
            .decode_mut(v.as_bytes(), &mut buf)
            .map_err(|_| StoreError::HashDecodePartialError {
                error: v.to_string(),
            })?; // TODO: error handling
                 //base64::decode_config_slice(v, base64::STANDARD, &mut buf)?;
        Ok(Hash::SHA256(buf.to_vec()))
    }
    pub fn is_sha256(&self) -> bool {
        match self {
            Hash::SHA256(_) => true,
            _ => false,
        }
    }
    pub fn from_sha256_vec(v: &[u8]) -> Result<Self, StoreError> {
        Ok(Hash::SHA256(v.to_vec()))
    }

    pub fn to_base32(&self) -> Result<String, StoreError> {
        match self {
            Hash::SHA256(v) => {
                let v = data_encoding::BASE32.encode(v);
                Ok(v)
            }
            Hash::Compressed(v) => {
                let v = base32::encode(v);
                Ok(v)
            }
            _ => Err(StoreError::BadArchive {
                msg: "base32 error".to_string(),
            }), // TODO: better error type
        }
    }

    pub fn compress_hash(self, len: usize) -> Result<Self, StoreError> {
        // TODO: only take a referenc, so no cloning is needed? (or even return another type?)
        //let mut vec=  Vec::with_capacity(len);
        let mut vec = vec![0; len];
        if let Hash::SHA256(v) = self {
            for i in 0..v.len() {
                vec[i % len] ^= v[i];
            }
        } else {
            return Err(StoreError::Unimplemented {
                msg: "compress_hash".to_string(),
            });
        }
        Ok(Hash::Compressed(vec))
    }

    pub fn to_sql_string(&self) -> String {
        // TODO: return StoreError for none?
        match self {
            Hash::SHA256(v) => format!("sha256:{}", data_encoding::HEXLOWER.encode(v)),
            _ => {
                panic!("unsupported hash type");
                "unsuported".to_string()
            }
        }
    }

    pub fn from_sql_string(s: &str) -> Result<Hash, StoreError> {
        let v: Vec<&str> = s.split(':').collect();
        // TOOD: len checking
        match *v.get(0).unwrap_or(&"") {
            "sha256" => {
                trace!("decoding sha hash: {}", v.get(1).unwrap());
                println!("decoding sha hash: {}", v.get(1).unwrap());
                let data =
                    data_encoding::HEXLOWER_PERMISSIVE.decode(v.get(1).unwrap().as_bytes())?;
                //BASE32.decode(v.get(1).unwrap().as_bytes())?;
                /*let mut buf: [u8; 32] = [0; 32];
                trace!("decoding sha hash: {}", v.get(1).unwrap());
                data_encoding::HEXLOWER
                    .decode_mut(v.get(1).unwrap().as_bytes(), &mut buf)
                    .map_err(|v| {
                        println!("Error: {:?}", v);
                        StoreError::HashDecodePartialError {
                            error: "hash thingy failed".to_string(),
                        }
                    })?;*/
                Ok(Hash::SHA256(data))
            }
            _ => Err(StoreError::HashDecodePartialError {
                error: "invalid hash type".to_string(),
            }),
        }
    }

    pub fn hash_string(s: &str) -> Result<Hash, StoreError> {
        // read hash type from s
        trace!("reading hash string: '{}'", s);
        let ht: Vec<&str> = s.split(':').collect();
        if ht.len() < 4 {
            unimplemented!("invalid hash string");
        }
        let ht_pos = ht.len() - 4;
        let ht = ht[ht_pos];
        match ht {
            "sha256" => Hash::hash_string_sha256(s),
            _ => unimplemented!("not sha256"),
        }
    }

    pub fn hash_string_sha256(s: &str) -> Result<Hash, StoreError> {
        trace!("hashing: '{}'", s);
        Hash::from_sha256_vec(ring::digest::digest(&ring::digest::SHA256, s.as_bytes()).as_ref())
    }
}

impl std::convert::TryFrom<&str> for Hash {
    type Error = StoreError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        trace!("making hash from '{}'", v);
        let v: Vec<&str> = v.split(':').collect();
        // TODO: len checking
        match *v.get(0).unwrap_or(&"") {
            "sha256" => {
                trace!("decoding sha hash: {}", v.get(1).unwrap());
                println!("decoding sha hash: {}", v.get(1).unwrap());
                let data = base32::decode(v.get(1).unwrap())?;
                //BASE32.decode(v.get(1).unwrap().as_bytes())?;
                /*let mut buf: [u8; 32] = [0; 32];
                trace!("decoding sha hash: {}", v.get(1).unwrap());
                data_encoding::HEXLOWER
                    .decode_mut(v.get(1).unwrap().as_bytes(), &mut buf)
                    .map_err(|v| {
                        println!("Error: {:?}", v);
                        StoreError::HashDecodePartialError {
                            error: "hash thingy failed".to_string(),
                        }
                    })?;*/
                Ok(Hash::SHA256(data))
            }
            _ => Err(StoreError::HashDecodePartialError {
                error: "invalid hash type".to_string(),
            }),
        }
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hash::SHA256(v) => write!(f, "{}", base32::encode(v)), // no sha256:<hash>??
            Hash::None => write!(f, "EMTPY-HASH"),
            Hash::Compressed(v) => {
                let s = base32::encode(v);
                write!(f, "{}", s)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Hash, StoreError};
    use std::convert::TryFrom;

    #[test]
    fn decode_encode() {
        let hash_1 =
            Hash::try_from("sha256:1yh3wfhqrgm27n60qbfdgmyv00z3bwvs8bcmy233cgqy2rq2s19r").unwrap();

        let string = hash_1.to_string();

        assert_eq!(
            "1yh3wfhqrgm27n60qbfdgmyv00z3bwvs8bcmy233cgqy2rq2s19r",
            string
        );
    }

    #[test]
    fn compress_hash() {
        let hash_1 =
            Hash::try_from("sha256:1yh3wfhqrgm27n60qbfdgmyv00z3bwvs8bcmy233cgqy2rq2s19r").unwrap();
        let hash_1 = hash_1.compress_hash(20).unwrap();
        let hash_1 = hash_1.to_string();

        let hash_2 = "gmyv00z3bwvs9mwn2ckvm0dw5gy22a7l";
        assert_eq!(hash_1, hash_2);
    }
}
