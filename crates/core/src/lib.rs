pub mod init;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum JogenError {
    #[error("Jogen project already exists at: {0}")]
    ProjectAlreadyExists(String),

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config Error: {0}")]
    Config(#[from] toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, JogenError>;
