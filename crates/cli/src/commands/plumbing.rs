use anyhow::{Context, Result};
use colored::*;
use jogen_core::indexer::Indexer;
use jogen_core::objects::directory::Directory;
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
