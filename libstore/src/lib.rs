pub mod archive;

pub mod build;

pub mod connection;

pub mod crypto;

pub mod error;

pub mod gc;

pub mod plugin;

pub mod source;

pub mod store;

pub use error::StoreError;
pub use store::Store;

use lazy_static::lazy_static;
lazy_static! {
    pub static ref CONFIG: std::sync::RwLock<libutil::config::NixConfig> =
        std::sync::RwLock::new(libutil::config::NixConfig::default());
}
