use crate::args::{InitArgs, SaveArgs};
use anyhow::Result;
use chrono::Utc;
use colored::*;
use jogen_core::{
    indexer::Indexer,
    object_store::{ObjectStore, ObjectType},
    objects::{snapshot::Snapshot, JogenObject},
    ref_store::RefStore,
};

pub fn handle(args: InitArgs) -> Result<()> {
    let _root = jogen_core::init::execute(args.path)?;

    println!("{}", "Jogen Project Initialized".green().bold());

    Ok(())
}

pub fn save(args: SaveArgs) -> Result<()> {
    let root_path = jogen_core::find_root_from_cwd()?;

    let objects_dir = root_path.join(".jogen").join("objects");

    let object_store = ObjectStore::new(objects_dir);
    let ref_store = RefStore::new(root_path.clone());

    println!("{}", "Scanning workspace...".dimmed());
    let indexer = Indexer::new(&object_store);
    let tree_hash = indexer
        .index_path(&root_path)?
        .ok_or_else(|| anyhow::anyhow!("Nothing to save (workspace is empty)"))?;

    let parent_hashes = match ref_store.read_head()? {
        Some(parent_hash) => vec![parent_hash],
        None => vec![],
    };

    let snapshot = Snapshot::new(
        tree_hash,
        parent_hashes.clone(),
        "Jogen User <user@jogen.com>".to_string(),
        Utc::now().timestamp(),
        args.context,
        args.message,
    );

    let snapshot_hash =
        object_store.write_object(snapshot.serialize()?.as_ref(), ObjectType::Snapshot)?;

    ref_store.update_head(&snapshot_hash)?;

    println!(
        "{} Saved snapshot {}",
        "✔".green(),
        snapshot_hash[..7].yellow()
    );

    if parent_hashes.is_empty() {
        println!("{}", "(Root Commit)".dimmed());
    } else {
        println!("Parent: {}", parent_hashes[0][..7].dimmed());
    }

    Ok(())
}
