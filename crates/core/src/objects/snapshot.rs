use std::borrow::Cow;
use std::fmt::{self, Write};

use crate::object_store::ObjectType;
use crate::objects::JogenObject;
use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotContext {
    Feature,
    Fix,
    Refactor,
    Docs,
    Chore,
    Merge,
    Initial,
}

impl SnapshotContext {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Fix => "fix",
            Self::Refactor => "refactor",
            Self::Docs => "docs",
            Self::Chore => "chore",
            Self::Merge => "merge",
            Self::Initial => "initial",
        }
    }
}

impl fmt::Display for SnapshotContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub directory_hash: String,
    pub parent_hashes: Vec<String>,
    pub author: String,
    pub timestamp: i64,
    pub context: SnapshotContext,
    pub message: String,
}

impl Snapshot {
    pub fn new(
        directory_hash: String,
        parent_hashes: Vec<String>,
        author: String,
        timestamp: i64,
        context: SnapshotContext,
        message: String,
    ) -> Self {
        Self {
            directory_hash,
            parent_hashes,
            author,
            timestamp,
            context,
            message,
        }
    }
}

impl JogenObject for Snapshot {
    fn object_type(&self) -> ObjectType {
        ObjectType::Snapshot
    }

    fn serialize(&self) -> Result<Cow<'_, [u8]>> {
        let capacity = 75
            + (self.parent_hashes.len() * 72)
            + (8 + self.author.len())
            + 26
            + (9 + self.context.as_str().len())
            + self.message.len()
            + 2;

        let mut out = String::with_capacity(capacity);

        out.push_str("directory ");
        out.push_str(&self.directory_hash);
        out.push('\n');

        for parent in &self.parent_hashes {
            out.push_str("parent ");
            out.push_str(parent);
            out.push('\n');
        }

        out.push_str("author ");
        out.push_str(&self.author);
        out.push('\n');

        let _ = writeln!(out, "time {}", self.timestamp);

        out.push_str("context ");
        out.push_str(self.context.as_str());
        out.push('\n');

        out.push('\n');
        out.push_str(&self.message);

        Ok(Cow::Owned(out.into_bytes()))
    }
}
