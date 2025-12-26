use anyhow::{Context, Result};
use chrono::Utc;
use colored::*;
use jogen_core::indexer::Indexer;
use jogen_core::objects::directory::Directory;
use jogen_core::objects::snapshot::{Snapshot, SnapshotContext};
use jogen_core::objects::JogenObject;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use jogen_core::object_store::{ObjectStore, ObjectType};

pub fn hash_object(file_path: PathBuf) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let content =
        fs::read(&file_path).with_context(|| format!("Could not read file: {:?}", file_path))?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let store = ObjectStore::new(objects_dir);

    let hash = store.write_object(&content, ObjectType::Blob)?;

    println!("{}", hash.cyan());

    Ok(())
}

pub fn cat_file(hash: String) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let store = ObjectStore::new(objects_dir);

    let (kind, content) = store.read_object(&hash)?;

    eprintln!("{} {}", "Type:".dimmed(), kind.to_string().yellow());

    io::stdout().write_all(&content)?;

    Ok(())
}

pub fn write_directory() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let store = ObjectStore::new(objects_dir.clone());
    let indexer = Indexer::new(&store);

    match indexer.index_path(&root_path)? {
        Some(hash) => println!("{}", hash.cyan()),
        None => println!("{}", "Nothing to save (empty project)".yellow()),
    }

    Ok(())
}

pub fn read_directory(hash: String) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let store = ObjectStore::new(objects_dir);

    let (kind, content) = store.read_object(&hash)?;

    if kind != ObjectType::Directory {
        return Err(anyhow::anyhow!(
            "Object {} is a {}, not a directory",
            hash,
            kind
        ));
    }

    let directory = Directory::parse(&content)?;

    for entry in directory.entries() {
        println!(
            "{} {} {}    {}",
            format!("{:06o}", entry.mode as u8).dimmed(),
            "blob",
            entry.hash.yellow(),
            entry.name
        );
    }

    Ok(())
}

pub fn write_snapshot() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let store = ObjectStore::new(objects_dir.clone());

    let indexer = Indexer::new(&store);

    let directory_hash = indexer
        .index_path(&root_path)?
        .ok_or_else(|| anyhow::anyhow!("Cannot snapshot an empty project"))?;

    println!("Directory Hash: {}", directory_hash.yellow());

    let snapshot = Snapshot::new(
        directory_hash,
        vec![],
        "Jogen User <user@jogen.com>".to_string(),
        Utc::now().timestamp(),
        SnapshotContext::Initial,
        "Snapshot created via plumbing command".to_string(),
    );

    let snapshot_hash = store.write_object(snapshot.serialize()?.as_ref(), ObjectType::Snapshot)?;

    println!("Snapshot Hash:  {}", snapshot_hash.green().bold());
    println!("\nTo verify: jogen cat-file {}", snapshot_hash);

    Ok(())
}

pub fn read_snapshot(hash: String) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let store = ObjectStore::new(objects_dir);

    let (kind, content) = store.read_object(&hash)?;

    if kind != ObjectType::Snapshot {
        return Err(anyhow::anyhow!(
            "Object {} is a {}, not a snapshot",
            hash,
            kind
        ));
    }

    let snapshot = Snapshot::deserialize(&content)?;

    println!("Snapshot Hash:   {}", hash.green().bold());
    println!("Directory Hash:  {}", snapshot.directory_hash.yellow());
    println!(
        "Context:         {}",
        format!("{:?}", snapshot.context).yellow()
    );
    println!("Author:          {}", snapshot.author.yellow());
    println!(
        "Timestamp:       {}",
        snapshot.timestamp.to_string().yellow()
    );
    println!("\nMessage:\n{}", snapshot.message);

    Ok(())
}

pub fn history() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let store = ObjectStore::new(objects_dir.clone());

    let mut current_hash = {
        let ref_store = jogen_core::ref_store::RefStore::new(root_path);
        ref_store
            .read_head()?
            .ok_or_else(|| anyhow::anyhow!("No snapshots found (head is empty)"))?
    };

    while !current_hash.is_empty() {
        let (kind, content) = store.read_object(&current_hash)?;

        if kind != ObjectType::Snapshot {
            return Err(anyhow::anyhow!(
                "Object {} is a {}, not a snapshot",
                current_hash,
                kind
            ));
        }

        let snapshot = Snapshot::deserialize(&content)?;

        println!("{} {}", "Snapshot:".dimmed(), current_hash.green().bold());
        println!("Author:    {}", snapshot.author.yellow());
        println!("Timestamp: {}", snapshot.timestamp.to_string().yellow());
        println!("Context:   {}", format!("{:?}", snapshot.context).yellow());
        println!("Message:   {}", snapshot.message);
        println!();

        if snapshot.parent_hashes.is_empty() {
            break;
        } else {
            current_hash = snapshot.parent_hashes[0].clone();
        }
    }

    Ok(())
}
