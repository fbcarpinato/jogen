use std::fs;
use std::path::Path;

use crate::object_store::ObjectStore;
use crate::objects::blob::Blob;
use crate::objects::directory::{Directory, DirectoryEntry, EntryMode};
use crate::objects::JogenObject;

use crate::{JogenError, Result};

pub struct Indexer<'a> {
    store: &'a ObjectStore,
}

impl<'a> Indexer<'a> {
    pub fn new(store: &'a ObjectStore) -> Self {
        Self { store }
    }

    pub fn index_path(&self, path: &Path) -> Result<Option<String>> {
        let metadata = fs::symlink_metadata(path).map_err(JogenError::Io)?;

        let file_name = path
            .file_name()
            .ok_or_else(|| {
                JogenError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "No filename",
                ))
            })?
            .to_string_lossy();

        if file_name == ".jogen" {
            return Ok(None);
        }

        if metadata.is_dir() {
            let mut directory = Directory::new();

            for entry in fs::read_dir(path).map_err(JogenError::Io)? {
                let entry = entry.map_err(JogenError::Io)?;
                let child_path = entry.path();
                let child_name = entry.file_name().to_string_lossy().to_string();

                if let Some(child_hash) = self.index_path(&child_path)? {
                    let child_meta = entry.metadata().map_err(JogenError::Io)?;
                    let mode = if child_meta.is_dir() {
                        EntryMode::Directory
                    } else {
                        EntryMode::File
                    };

                    directory.add_entry(DirectoryEntry {
                        mode,
                        name: child_name,
                        hash: child_hash,
                    });
                }
            }

            let hash = self
                .store
                .write_object(directory.serialize()?.as_ref(), directory.object_type())?;

            return Ok(Some(hash));
        }

        if metadata.is_file() {
            let content = fs::read(path).map_err(JogenError::Io)?;

            let blob = Blob::new(content);

            let hash = self
                .store
                .write_object(blob.serialize()?.as_ref(), blob.object_type())?;

            return Ok(Some(hash));
        }

        Ok(None)
    }
}
