use std::borrow::Cow;

use crate::object_store::ObjectType;
use crate::objects::JogenObject;
use crate::{JogenError, Result};

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[repr(u8)]
pub enum EntryMode {
    File = 0o1,       // Internal ID 1
    Executable = 0o2, // Internal ID 2
    Directory = 0o4,  // Internal ID 4
}

impl TryFrom<u8> for EntryMode {
    type Error = JogenError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0o1 => Ok(EntryMode::File),
            0o2 => Ok(EntryMode::Executable),
            0o4 => Ok(EntryMode::Directory),
            _ => Err(JogenError::InvalidEntryMode(value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DirectoryEntry {
    pub mode: EntryMode,
    pub name: String,
    pub hash: String,
}

impl Ord for DirectoryEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}
impl PartialOrd for DirectoryEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Directory {
    entries: Vec<DirectoryEntry>,
}

impl Directory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: DirectoryEntry) {
        self.entries.push(entry);
    }
}

impl JogenObject for Directory {
    fn object_type(&self) -> ObjectType {
        ObjectType::Directory
    }

    fn serialize(&self) -> Result<Cow<'_, [u8]>> {
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort();

        let mut content = Vec::new();

        for entry in sorted_entries {
            let mode_str = match entry.mode {
                EntryMode::File => "100644",
                EntryMode::Executable => "100755",
                EntryMode::Directory => "040000",
            };
            content.extend_from_slice(mode_str.as_bytes());

            content.push(b' ');

            content.extend_from_slice(entry.name.as_bytes());

            content.push(0);

            let hash_bytes = hex::decode(&entry.hash).map_err(|_| {
                JogenError::ObjectCorrupt(format!("Invalid hex hash for {}", entry.name))
            })?;
            content.extend_from_slice(&hash_bytes);
        }

        Ok(Cow::Owned(content))
    }
}
