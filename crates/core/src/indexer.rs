use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

use crate::object_store::ObjectStore;
use crate::objects::blob::Blob;
use crate::objects::directory::{Directory, DirectoryEntry, EntryMode};
use crate::objects::JogenObject;

use crate::{JogenError, Result};

pub struct Indexer<'a> {
    store: &'a ObjectStore,
    ignore: Gitignore,
    root_path: PathBuf,
}

impl<'a> Indexer<'a> {
    pub fn new(store: &'a ObjectStore, root_path: &Path) -> Self {
        let mut builder = GitignoreBuilder::new(root_path);
        let jogenignore = root_path.join(".jogenignore");
        if jogenignore.exists() {
            let _ = builder.add(jogenignore);
        }
        let ignore = builder.build().unwrap_or_else(|_| Gitignore::empty());

        Self {
            store,
            ignore,
            root_path: root_path.to_path_buf(),
        }
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

        let relative_path = path.strip_prefix(&self.root_path).unwrap_or(path);
        if self.ignore.matched(relative_path, metadata.is_dir()).is_ignore() {
            return Ok(None);
        }

        if metadata.is_dir() {
            let entries: Vec<_> = fs::read_dir(path).map_err(JogenError::Io)?.collect();

            let child_results: Result<Vec<Option<DirectoryEntry>>> = entries
                .into_par_iter()
                .map(|entry_res| {
                    let entry = entry_res.map_err(JogenError::Io)?;
                    let child_path = entry.path();
                    let child_name = entry.file_name().to_string_lossy().to_string();

                    if let Some(child_hash) = self.index_path(&child_path)? {
                        let child_meta = entry.metadata().map_err(JogenError::Io)?;
                        let mode = if child_meta.is_dir() {
                            EntryMode::Directory
                        } else {
                            EntryMode::File
                        };

                        Ok(Some(DirectoryEntry {
                            mode,
                            name: child_name,
                            hash: child_hash,
                        }))
                    } else {
                        Ok(None)
                    }
                })
                .collect();

            let mut directory = Directory::new();
            for child_opt in child_results? {
                if let Some(child) = child_opt {
                    directory.add_entry(child);
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
