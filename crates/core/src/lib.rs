pub mod init;
pub mod object_store;

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JogenError {
    // --- Project Structure Errors ---
    #[error("A Jogen Project already exists at: {0}")]
    ProjectExists(String),

    #[error("Could not locate a Jogen Project in this directory or any parent.")]
    ProjectRootNotFound,

    // --- Object Store Errors ---
    #[error("Object not found in store: {0}")]
    ObjectNotFound(String),

    #[error("Object is corrupt or has invalid header: {0}")]
    ObjectCorrupt(String),

    // --- System Errors ---
    #[error("Input/Output Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration Error: {0}")]
    Config(#[from] toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, JogenError>;

pub fn find_root(start_path: &Path) -> Result<PathBuf> {
    let mut current_path = start_path;

    loop {
        let jogen_dir = current_path.join(".jogen");

        if jogen_dir.exists() && jogen_dir.is_dir() {
            return Ok(current_path.to_path_buf());
        }

        match current_path.parent() {
            Some(parent) => current_path = parent,
            None => {
                return Err(JogenError::ProjectRootNotFound);
            }
        }
    }
}
