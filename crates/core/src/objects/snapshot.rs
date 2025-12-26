use std::borrow::Cow;
use std::fmt::{self, Write};

use crate::object_store::ObjectType;
use crate::objects::JogenObject;
use crate::Result;
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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
        write!(f, "{}", self.as_str())
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

impl Snapshot {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let content = String::from_utf8_lossy(data);
        let mut lines = content.lines();

        let mut directory_hash = String::new();
        let mut parent_hashes = Vec::new();
        let mut author = String::new();
        let mut timestamp = 0i64;
        let mut context = SnapshotContext::Chore;
        let mut message_lines = Vec::new();

        while let Some(line) = lines.next() {
            if line.is_empty() {
                break;
            }

            let mut parts = line.splitn(2, ' ');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");

            match key {
                "directory" => directory_hash = value.to_string(),
                "parent" => parent_hashes.push(value.to_string()),
                "author" => author = value.to_string(),
                "time" => timestamp = value.parse().unwrap_or(0),
                "context" => {
                    context = match value {
                        "feature" => SnapshotContext::Feature,
                        "fix" => SnapshotContext::Fix,
                        "refactor" => SnapshotContext::Refactor,
                        "docs" => SnapshotContext::Docs,
                        "chore" => SnapshotContext::Chore,
                        "merge" => SnapshotContext::Merge,
                        "initial" => SnapshotContext::Initial,
                        _ => SnapshotContext::Chore,
                    }
                }
                _ => {}
            }
        }

        for line in lines {
            message_lines.push(line);
        }

        let message = message_lines.join("\n");

        Ok(Snapshot {
            directory_hash,
            parent_hashes,
            author,
            timestamp,
            context,
            message,
        })
    }
}
