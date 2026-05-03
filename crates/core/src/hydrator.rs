use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::object_store::ObjectStore;
use crate::objects::directory::{Directory, EntryMode};
use crate::{JogenError, Result};

pub struct Hydrator<'a> {
    store: &'a ObjectStore,
}

impl<'a> Hydrator<'a> {
    pub fn new(store: &'a ObjectStore) -> Self {
        Self { store }
    }

    pub fn apply_diff(
        &self,
        old_tree_hash: &str,
        new_tree_hash: &str,
        current_path: &Path,
    ) -> Result<()> {
        if old_tree_hash == new_tree_hash {
            return Ok(());
        }

        // 1. Parse both directory structures
        let old_dir = self.load_directory(old_tree_hash)?;
        let new_dir = self.load_directory(new_tree_hash)?;

        // 2. Index the OLD directory for fast lookups
        let old_map: HashMap<String, _> = old_dir
            .entries()
            .iter()
            .map(|e| (e.name.clone(), e))
            .collect();

        // 3. Collect entries to process so we can do it in parallel
        let new_entries: Vec<_> = new_dir.entries().iter().collect();

        // Separate handling into parallel writes/updates
        new_entries.into_par_iter().try_for_each(|new_entry| -> Result<()> {
            let child_path = current_path.join(&new_entry.name);
            
            // We read from the shared old_map, which is fine since we aren't mutating it here.
            match old_map.get(&new_entry.name) {
                Some(&old_entry) => {
                    // Entry exists in both old and new.
                    if old_entry.hash == new_entry.hash && old_entry.mode == new_entry.mode {
                        return Ok(());
                    }

                    if old_entry.mode == EntryMode::Directory
                        && new_entry.mode == EntryMode::Directory
                    {
                        self.apply_diff(&old_entry.hash, &new_entry.hash, &child_path)?;
                    } else {
                        // Types differ or file content changed. Overwrite the old entry.
                        if old_entry.mode == EntryMode::Directory {
                            fs::remove_dir_all(&child_path).map_err(JogenError::Io)?;
                        } else {
                            fs::remove_file(&child_path).map_err(JogenError::Io)?;
                        }

                        if new_entry.mode == EntryMode::Directory {
                            self.hydrate_directory(&new_entry.hash, &child_path)?;
                        } else {
                            self.write_blob(&new_entry.hash, &child_path, new_entry.mode)?;
                        }
                    }
                }
                None => {
                    // Entry is new. Create it.
                    if new_entry.mode == EntryMode::Directory {
                        self.hydrate_directory(&new_entry.hash, &child_path)?;
                    } else {
                        self.write_blob(&new_entry.hash, &child_path, new_entry.mode)?;
                    }
                }
            }
            Ok(())
        })?;

        // 4. Any entry remaining in `old_map` was not in `new_dir`, so delete it.
        // We do this serially since deletes are generally fast, but we need to check if it's missing.
        let new_names: std::collections::HashSet<_> = new_dir.entries().iter().map(|e| e.name.as_str()).collect();
        for (name, entry) in old_map {
            if !new_names.contains(name.as_str()) {
                let child_path = current_path.join(name);
                if entry.mode == EntryMode::Directory {
                    fs::remove_dir_all(child_path).map_err(JogenError::Io)?;
                } else {
                    fs::remove_file(child_path).map_err(JogenError::Io)?;
                }
            }
        }

        Ok(())
    }

    /// Recursively writes a directory tree from the object store to the filesystem.
    pub fn hydrate_directory(&self, tree_hash: &str, path: &Path) -> Result<()> {
        let dir = self.load_directory(tree_hash)?;
        if !path.exists() {
            fs::create_dir_all(path).map_err(JogenError::Io)?;
        }

        dir.entries().into_par_iter().try_for_each(|entry| -> Result<()> {
            let child = path.join(&entry.name);
            if entry.mode == EntryMode::Directory {
                self.hydrate_directory(&entry.hash, &child)?;
            } else {
                self.write_blob(&entry.hash, &child, entry.mode)?;
            }
            Ok(())
        })?;
        
        Ok(())
    }

    /// Parses a directory object from the store.
    fn load_directory(&self, hash: &str) -> Result<Directory> {
        let (kind, content) = self.store.read_object(hash)?;
        if kind != crate::object_store::ObjectType::Directory {
            return Err(JogenError::ObjectCorrupt(format!(
                "Expected Dir, found {}",
                kind
            )));
        }
        Directory::parse(&content)
    }

    /// Writes a blob from the object store to a file on the filesystem.
    fn write_blob(&self, hash: &str, path: &Path, mode: EntryMode) -> Result<()> {
        let (_, content) = self.store.read_object(hash)?;
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }

        fs::write(path, content)?;

        #[cfg(unix)]
        if mode == EntryMode::Executable {
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms)?;
        }
        Ok(())
    }
}
