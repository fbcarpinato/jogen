use std::collections::{HashMap, HashSet};

use crate::object_store::{ObjectStore, ObjectType};
use crate::objects::directory::{Directory, DirectoryEntry, EntryMode};
use crate::objects::blob::Blob;
use crate::objects::JogenObject;

pub struct MergeResult {
    pub tree_hash: Option<String>,
    pub conflicts: Vec<MergeConflict>,
}

pub struct MergeConflict {
    pub path: String,
    pub incoming: MergeConflictIncoming,
}

pub enum MergeConflictIncoming {
    BlobHash(String),
    Deleted,
}

pub struct MergeEngine<'a> {
    store: &'a ObjectStore,
}

impl<'a> MergeEngine<'a> {
    pub fn new(store: &'a ObjectStore) -> Self {
        Self { store }
    }

    /// Merges three directory trees in memory.
    /// Returns a MergeResult containing the partially merged tree (keeping Head for conflicts)
    /// and a list of conflicted paths with their incoming target hashes.
    pub fn merge_trees(
        &self,
        base_hash: Option<&str>,
        head_hash: Option<&str>,
        target_hash: Option<&str>,
        current_path: &str,
    ) -> MergeResult {
        if head_hash == target_hash {
            return MergeResult {
                tree_hash: head_hash.map(|s| s.to_string()),
                conflicts: vec![],
            };
        }

        let base_dir = self.load_directory_opt(base_hash);
        let head_dir = self.load_directory_opt(head_hash);
        let target_dir = self.load_directory_opt(target_hash);

        let mut base_map = self.map_entries(&base_dir);
        let mut head_map = self.map_entries(&head_dir);
        let mut target_map = self.map_entries(&target_dir);

        let mut all_names = HashSet::new();
        all_names.extend(base_map.keys().cloned());
        all_names.extend(head_map.keys().cloned());
        all_names.extend(target_map.keys().cloned());

        let mut merged_dir = Directory::new();
        let mut conflicts = Vec::new();

        for name in all_names {
            let base_entry = base_map.remove(&name);
            let head_entry = head_map.remove(&name);
            let target_entry = target_map.remove(&name);

            let path = if current_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", current_path, name)
            };

            // If head and target agree, use that
            if self.entries_eq(&head_entry, &target_entry) {
                if let Some(entry) = head_entry {
                    merged_dir.add_entry(entry);
                }
                continue;
            }

            // If head is same as base, target changed it
            if self.entries_eq(&base_entry, &head_entry) {
                if let Some(entry) = target_entry {
                    merged_dir.add_entry(entry);
                }
                continue;
            }

            // If target is same as base, head changed it
            if self.entries_eq(&base_entry, &target_entry) {
                if let Some(entry) = head_entry {
                    merged_dir.add_entry(entry);
                }
                continue;
            }

            // Both changed it differently
            match (&head_entry, &target_entry) {
                (Some(h), Some(t)) if h.mode == EntryMode::Directory && t.mode == EntryMode::Directory => {
                    let b_hash = base_entry.as_ref().map(|e| e.hash.as_str());
                    let mut sub_result = self.merge_trees(b_hash, Some(&h.hash), Some(&t.hash), &path);
                    
                    if let Some(merged_hash) = sub_result.tree_hash {
                        merged_dir.add_entry(DirectoryEntry {
                            name,
                            mode: EntryMode::Directory,
                            hash: merged_hash,
                        });
                    }
                    conflicts.append(&mut sub_result.conflicts);
                }
                _ => {
                    // Try auto-merging first if both are files
                    let auto_merged = if let (Some(b), Some(h), Some(t)) = (&base_entry, &head_entry, &target_entry) {
                        if h.mode == EntryMode::File && t.mode == EntryMode::File && b.mode == EntryMode::File {
                            if let (Ok((_, b_content)), Ok((_, h_content)), Ok((_, t_content))) = (
                                self.store.read_object(&b.hash),
                                self.store.read_object(&h.hash),
                                self.store.read_object(&t.hash)
                            ) {
                                if let (Ok(b_str), Ok(h_str), Ok(t_str)) = (
                                    std::str::from_utf8(&b_content),
                                    std::str::from_utf8(&h_content),
                                    std::str::from_utf8(&t_content)
                                ) {
                                    let merge_opts = diffy::MergeOptions::new();
                                    if let Ok(merged_str) = merge_opts.merge(b_str, h_str, t_str) {
                                        // Auto merge succeeded, write to store
                                        let blob = Blob::new(merged_str.into_bytes());
                                        if let Ok(serialized) = blob.serialize() {
                                            if let Ok(merged_hash) = self.store.write_object(
                                                serialized.as_ref(),
                                                ObjectType::Blob
                                            ) {
                                                Some(merged_hash)
                                            } else { None }
                                        } else { None }
                                    } else { None }
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None };

                    if let Some(merged_hash) = auto_merged {
                        merged_dir.add_entry(DirectoryEntry {
                            name,
                            mode: EntryMode::File,
                            hash: merged_hash,
                        });
                    } else {
                        // Actual conflict (file vs file text conflict, file vs dir, both modified file differently)
                        // Keep HEAD's version in the merged tree
                        if let Some(h) = head_entry {
                            merged_dir.add_entry(h);
                        }
                        
                        // Record incoming side so the Hydrator can create a conflict marker.
                        if let Some(t) = target_entry {
                            conflicts.push(MergeConflict {
                                path,
                                incoming: MergeConflictIncoming::BlobHash(t.hash),
                            });
                        } else {
                            conflicts.push(MergeConflict {
                                path,
                                incoming: MergeConflictIncoming::Deleted,
                            });
                        }
                    }
                }
            }
        }

        let tree_hash = if let Ok(serialized) = merged_dir.serialize() {
            self.store
                .write_object(serialized.as_ref(), merged_dir.object_type())
                .ok()
        } else {
            None
        };

        MergeResult {
            tree_hash,
            conflicts,
        }
    }

    fn entries_eq(&self, a: &Option<DirectoryEntry>, b: &Option<DirectoryEntry>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => a.hash == b.hash && a.mode == b.mode,
            (None, None) => true,
            _ => false,
        }
    }

    fn map_entries(&self, dir: &Option<Directory>) -> HashMap<String, DirectoryEntry> {
        let mut map = HashMap::new();
        if let Some(d) = dir {
            for entry in d.entries() {
                map.insert(entry.name.clone(), entry.clone());
            }
        }
        map
    }

    fn load_directory_opt(&self, hash: Option<&str>) -> Option<Directory> {
        let hash = hash?;
        let (kind, content) = self.store.read_object(hash).ok()?;
        if kind == ObjectType::Directory {
            Directory::parse(&content).ok()
        } else {
            None
        }
    }
}
