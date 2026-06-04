pub mod crypto;
pub mod db;
pub mod entry;
pub mod vault;

pub use entry::{Entry, EntryType, MetadataValue};
pub use vault::{EntrySummary, Vault};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Wrong password")]
    WrongPassword,
    #[error("Vault is locked")]
    Locked,
}
