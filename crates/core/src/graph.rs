use std::collections::{HashSet, VecDeque};

use crate::object_store::{ObjectStore, ObjectType};
use crate::objects::snapshot::Snapshot;
use crate::{JogenError, Result};

pub struct GraphTraversal<'a> {
    store: &'a ObjectStore,
}

impl<'a> GraphTraversal<'a> {
    pub fn new(store: &'a ObjectStore) -> Self {
        Self { store }
    }

    /// Finds the lowest common ancestor (LCA) snapshot hash between two snapshot hashes.
    pub fn find_common_ancestor(&self, head_a: &str, head_b: &str) -> Result<Option<String>> {
        if head_a == head_b {
            return Ok(Some(head_a.to_string()));
        }

        let mut queue_a = VecDeque::new();
        let mut queue_b = VecDeque::new();
        
        let mut visited_a = HashSet::new();
        let mut visited_b = HashSet::new();

        queue_a.push_back(head_a.to_string());
        visited_a.insert(head_a.to_string());

        queue_b.push_back(head_b.to_string());
        visited_b.insert(head_b.to_string());

        while !queue_a.is_empty() || !queue_b.is_empty() {
            // Process one layer of A
            if let Some(curr_a) = queue_a.pop_front() {
                if visited_b.contains(&curr_a) {
                    return Ok(Some(curr_a));
                }

                if let Ok(snapshot) = self.load_snapshot(&curr_a) {
                    for parent in snapshot.parent_hashes {
                        if visited_a.insert(parent.clone()) {
                            queue_a.push_back(parent);
                        }
                    }
                }
            }

            // Process one layer of B
            if let Some(curr_b) = queue_b.pop_front() {
                if visited_a.contains(&curr_b) {
                    return Ok(Some(curr_b));
                }

                if let Ok(snapshot) = self.load_snapshot(&curr_b) {
                    for parent in snapshot.parent_hashes {
                        if visited_b.insert(parent.clone()) {
                            queue_b.push_back(parent);
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn load_snapshot(&self, hash: &str) -> Result<Snapshot> {
        let (kind, content) = self.store.read_object(hash)?;
        if kind != ObjectType::Snapshot {
            return Err(JogenError::ObjectCorrupt(format!(
                "Expected Snapshot, found {}",
                kind
            )));
        }
        Snapshot::deserialize(&content)
    }
}
