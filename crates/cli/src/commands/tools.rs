use anyhow::{Context, Result};
use chrono::Utc;
use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::commands::JogenRepo;

use jogen_core::{
    indexer::Indexer,
    object_store::ObjectType,
    objects::{
        directory::Directory,
        snapshot::{Snapshot, SnapshotContext},
        JogenObject,
    },
};

pub fn hash_object(file_path: PathBuf) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let content =
        fs::read(&file_path).with_context(|| format!("Could not read file: {:?}", file_path))?;

    let hash = repo.object_store.write_object(&content, ObjectType::Blob)?;

    println!("{}", hash.cyan());

    Ok(())
}

pub fn cat_file(hash: String) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let (kind, content) = repo.object_store.read_object(&hash)?;

    eprintln!("{} {}", "Type:".dimmed(), kind.to_string().yellow());

    io::stdout().write_all(&content)?;

    Ok(())
}

pub fn write_directory() -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let indexer = Indexer::new(&repo.object_store);

    match indexer.index_path(&repo.root_path)? {
        Some(hash) => println!("{}", hash.cyan()),
        None => println!("{}", "Nothing to save (empty project)".yellow()),
    }

    Ok(())
}

pub fn read_directory(hash: String) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let (kind, content) = repo.object_store.read_object(&hash)?;

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
    let repo = JogenRepo::from_cwd()?;

    let indexer = Indexer::new(&repo.object_store);

    let directory_hash = indexer
        .index_path(&repo.root_path)?
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

    let snapshot_hash = repo
        .object_store
        .write_object(snapshot.serialize()?.as_ref(), ObjectType::Snapshot)?;

    println!("Snapshot Hash:  {}", snapshot_hash.green().bold());
    println!("\nTo verify: jogen cat-file {}", snapshot_hash);

    Ok(())
}

pub fn read_snapshot(hash: String) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let (kind, content) = repo.object_store.read_object(&hash)?;

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
    let repo = JogenRepo::from_cwd()?;

    let mut current_hash = {
        let ref_store = jogen_core::ref_store::RefStore::new(repo.root_path);
        ref_store
            .read_head()?
            .ok_or_else(|| anyhow::anyhow!("No snapshots found (head is empty)"))?
    };

    while !current_hash.is_empty() {
        let (kind, content) = repo.object_store.read_object(&current_hash)?;

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
