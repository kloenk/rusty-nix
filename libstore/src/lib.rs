pub mod connection;
pub mod error;

pub mod store;

pub mod crypto;

pub mod archive;

pub mod reader;

pub mod build;

pub mod gc;

pub use store::open_store;
pub use store::Store;

use lazy_static::lazy_static;
lazy_static! {
    pub static ref CONFIG: std::sync::RwLock<libutil::config::NixConfig> =
        std::sync::RwLock::new(libutil::config::NixConfig::default());
}
