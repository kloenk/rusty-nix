use log::{trace, debug, warn, info};

use ring::signature::UnparsedPublicKey;

use std::collections::HashMap;

use super::error::StoreError;

pub struct PublicKey {
  pub name: String,
  pub key: String,
}

impl PublicKey {
  pub fn new(name: String, key: String) -> Self {
    Self {
      name, key,
    }
  }

  pub fn verify() -> Result<bool, StoreError> {

    unimplemented!()
  }
}

impl std::convert::TryFrom<&str> for PublicKey {
  type Error = super::error::StoreError;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    let v: Vec<&str> = value.split(':').collect();
    if v.len() != 2 {
      return Err(StoreError::InvalidKey{ key: value.to_string() });
    }

    Ok( Self {
      name: v[0].to_string(),
      key: v[1].to_string(),
    } )
  }
}

pub struct PublicKeys(HashMap<String, PublicKey>);

impl AsRef<HashMap<String, PublicKey>> for PublicKeys {
  fn as_ref(&self) -> &HashMap<String, PublicKey> {
      &self.0
  }
}

impl std::convert::TryFrom<Vec<String>> for PublicKeys {
  type Error = StoreError;

  fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
      let keys: Vec<Result<PublicKey, StoreError>> = value.iter().map(|v| { PublicKey::try_from(v.as_str()) } ).collect();
      
      let mut map = HashMap::new();

      for v in keys.into_iter() {
        let v = v?;
        map.insert(v.name.clone(), v);
      }

      Ok (Self(map))
  } 
}