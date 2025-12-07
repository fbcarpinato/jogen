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

    pub fn entries(&self) -> &Vec<DirectoryEntry> {
        &self.entries
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut entries = Vec::new();
        let mut cursor = 0;
        let len = data.len();

        while cursor < len {
            let space_idx = data[cursor..]
                .iter()
                .position(|&b| b == b' ')
                .ok_or_else(|| JogenError::ObjectCorrupt("Missing space after mode".into()))?
                + cursor;

            let mode_bytes = &data[cursor..space_idx];
            let mode_str = std::str::from_utf8(mode_bytes)
                .map_err(|_| JogenError::ObjectCorrupt("Invalid mode string".into()))?;

            let mode = match mode_str {
                "100644" => EntryMode::File,
                "100755" => EntryMode::Executable,
                "040000" => EntryMode::Directory,
                _ => {
                    return Err(JogenError::ObjectCorrupt(format!(
                        "Unknown mode: {}",
                        mode_str
                    )))
                }
            };

            cursor = space_idx + 1;

            let null_idx = data[cursor..].iter().position(|&b| b == 0).ok_or_else(|| {
                JogenError::ObjectCorrupt("Missing null terminator for name".into())
            })? + cursor;

            let name_bytes = &data[cursor..null_idx];
            let name = std::str::from_utf8(name_bytes)
                .map_err(|_| JogenError::ObjectCorrupt("Invalid UTF-8 filename".into()))?
                .to_string();

            cursor = null_idx + 1;

            if cursor + 32 > len {
                return Err(JogenError::ObjectCorrupt("Truncated hash bytes".into()));
            }
            let hash_bytes = &data[cursor..cursor + 32];
            let hash = hex::encode(hash_bytes); // Convert back to hex for the struct

            cursor += 32;

            entries.push(DirectoryEntry { mode, name, hash });
        }

        Ok(Self { entries })
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
