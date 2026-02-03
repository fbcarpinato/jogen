use anyhow::Result;
use chrono::Utc;
use colored::*;

use crate::{
    args::{InitArgs, SaveArgs},
    commands::JogenRepo,
};

use jogen_core::{
    indexer::Indexer,
    object_store::ObjectType,
    objects::{snapshot::Snapshot, JogenObject},
};

pub fn handle(args: InitArgs) -> Result<()> {
    let _root = jogen_core::init::execute(args.path)?;

    println!("{}", "Jogen Project Initialized".green().bold());

    Ok(())
}

pub fn save(args: SaveArgs) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    println!("{}", "Scanning workspace...".dimmed());
    let indexer = Indexer::new(&repo.object_store);
    let tree_hash = indexer
        .index_path(&repo.root_path)?
        .ok_or_else(|| anyhow::anyhow!("Nothing to save (workspace is empty)"))?;

    let parent_hashes = match repo.ref_store.read_head()? {
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

    let snapshot_hash = repo
        .object_store
        .write_object(snapshot.serialize()?.as_ref(), ObjectType::Snapshot)?;

    repo.ref_store.update_head(&snapshot_hash)?;

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

pub fn checkout(target_snapshot_hash: String) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;
    let hydrator = jogen_core::hydrator::Hydrator::new(&repo.object_store);

    println!(
        "{} Checking out snapshot {}",
        "↻".blue(),
        target_snapshot_hash[..7].yellow()
    );

    let current_snapshot_hash = repo
        .ref_store
        .read_head()?
        .ok_or_else(|| anyhow::anyhow!("No snapshots found. Cannot checkout."))?;

    let (_, content) = repo.object_store.read_object(&current_snapshot_hash)?;
    let current_snapshot = Snapshot::deserialize(&content)?;
    let head_tree_hash = current_snapshot.directory_hash;

    let indexer = Indexer::new(&repo.object_store);
    let workspace_tree_hash = indexer
        .index_path(&repo.root_path)?
        .ok_or_else(|| anyhow::anyhow!("Workspace is empty"))?;

    if workspace_tree_hash != head_tree_hash {
        return Err(anyhow::anyhow!(
            "Uncommitted changes found in workspace.\nHEAD tree: {}\nWorkspace: {}\nPlease save or discard changes.",
            head_tree_hash, workspace_tree_hash
        ));
    }

    if !repo.object_store.exists(&target_snapshot_hash) {
        return Err(anyhow::anyhow!(
            "Target snapshot {} not found.",
            target_snapshot_hash
        ));
    }

    let (kind, content) = repo.object_store.read_object(&target_snapshot_hash)?;
    let target_tree_hash = if kind == jogen_core::object_store::ObjectType::Snapshot {
        let snapshot = Snapshot::deserialize(&content)?;
        snapshot.directory_hash
    } else {
        return Err(anyhow::anyhow!(
            "Target {} is a {} object, not a snapshot.",
            target_snapshot_hash,
            kind
        ));
    };

    hydrator.apply_diff(&head_tree_hash, &target_tree_hash, &repo.root_path)?;

    repo.ref_store.update_head(&target_snapshot_hash)?;

    println!("{} Checkout complete", "✔".green());

    Ok(())
}

pub fn create_track(name: String) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let current_hash = repo
        .ref_store
        .read_head()?
        .ok_or_else(|| anyhow::anyhow!("Cannot create track: History is empty."))?;

    repo.ref_store.create_track(&name, &current_hash)?;
    println!("{} Created track {}", "✔".green(), name.yellow());

    Ok(())
}

pub fn list_tracks() -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let current = repo.ref_store.current_track()?;
    let tracks = repo.ref_store.list_tracks()?;

    if tracks.is_empty() {
        println!("{}", "No tracks found.".dimmed());
    }

    for track in tracks {
        if Some(&track) == current.as_ref() {
            println!("* {} {}", track.yellow(), "(current)".dimmed());
        } else {
            println!("  {}", track);
        }
    }

    Ok(())
}
