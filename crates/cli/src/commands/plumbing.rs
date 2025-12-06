use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use jogen_core::object_store::{ObjectStore, ObjectType};

pub fn hash_object(file_path: PathBuf) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let content =
        fs::read(&file_path).with_context(|| format!("Could not read file: {:?}", file_path))?;

    let store = ObjectStore::new(root_path);

    let hash = store.write_object(&content, ObjectType::Blob)?;

    println!("{}", hash.cyan());

    Ok(())
}

pub fn cat_file(hash: String) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root_path = jogen_core::find_root(&current_dir)?;

    let store = ObjectStore::new(root_path);

    let (kind, content) = store.read_object(&hash)?;

    eprintln!("{} {}", "Type:".dimmed(), kind.to_string().yellow());

    io::stdout().write_all(&content)?;

    Ok(())
}
