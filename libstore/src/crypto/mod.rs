use log::{debug, info, trace, warn};

use ring::signature::{UnparsedPublicKey, ED25519};

use std::collections::HashMap;

use super::error::StoreError;

#[derive(Debug)]
pub struct PublicKey {
    pub name: String,
    pub key: String,
}

impl PublicKey {
    pub fn new(name: String, key: String) -> Self {
        Self { name, key }
    }

    pub fn to_publickey(&self) -> ring::signature::UnparsedPublicKey<&str> {
        UnparsedPublicKey::new(&ED25519, self.key.as_str())
    }
}

impl std::convert::TryFrom<&str> for PublicKey {
    type Error = super::error::StoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let v: Vec<&str> = value.split(':').collect();
        if v.len() != 2 {
            return Err(StoreError::InvalidKey {
                key: value.to_string(),
            });
        }

        Ok(Self {
            name: v[0].to_string(),
            key: v[1].to_string(),
        })
    }
}

#[derive(Debug)]
pub struct PublicKeys(HashMap<String, PublicKey>);

impl PublicKeys {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn verify(&self, message: &[u8], sig: &str) -> Result<bool, StoreError> {
        let ss: Vec<&str> = sig.split(":").collect();

        if ss.len() != 2 {
            return Err(StoreError::InvalidKey {
                key: sig.to_string(),
            });
        }

        let name: &str = ss[0];
        let sig: &str = ss[1];

        let sig = data_encoding::BASE64.decode(sig.as_bytes())?; // base64?

        let key = self.as_ref().get(name);

        if let None = key {
            return Ok(false);
        }

        let key = key.unwrap().to_publickey();

        Ok(key.verify(message, &sig).is_ok())
    }
}

impl AsRef<HashMap<String, PublicKey>> for PublicKeys {
    fn as_ref(&self) -> &HashMap<String, PublicKey> {
        &self.0
    }
}

impl std::convert::TryFrom<Vec<String>> for PublicKeys {
    type Error = StoreError;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        let keys: Vec<Result<PublicKey, StoreError>> = value
            .iter()
            .map(|v| PublicKey::try_from(v.as_str()))
            .collect();

        let mut map = HashMap::new();

        for v in keys.into_iter() {
            let v = v?;
            map.insert(v.name.clone(), v);
        }

        Ok(Self(map))
    }
}
