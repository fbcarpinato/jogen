use anyhow::Result;
use chrono::Utc;
use colored::*;

use crate::{
    args::{InitArgs, SnapshotArgs},
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

pub fn snapshot(args: SnapshotArgs) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    println!("{}", "Scanning workspace...".dimmed());
    let indexer = Indexer::new(&repo.object_store);
    let tree_hash = indexer
        .index_path(&repo.root_path)?
        .ok_or_else(|| anyhow::anyhow!("Nothing to snapshot (workspace is empty)"))?;

    let parent_hashes = match repo.ref_store.read_head()? {
        Some(parent_hash) => vec![parent_hash],
        None => vec![],
    };

    let snapshot_obj = Snapshot::new(
        tree_hash,
        parent_hashes.clone(),
        "Jogen User <user@jogen.com>".to_string(),
        Utc::now().timestamp(),
        args.context,
        args.message,
    );

    let snapshot_hash = repo
        .object_store
        .write_object(snapshot_obj.serialize()?.as_ref(), ObjectType::Snapshot)?;

    repo.ref_store.update_head(&snapshot_hash)?;

    println!(
        "{} Created snapshot {}",
        "✔".green(),
        snapshot_hash[..7].yellow()
    );

    if parent_hashes.is_empty() {
        println!("{}", "(Root Snapshot)".dimmed());
    } else {
        println!("Parent: {}", parent_hashes[0][..7].dimmed());
    }

    Ok(())
}

pub fn status() -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let current_track = repo.ref_store.current_track()?;
    let head_hash = repo.ref_store.read_head()?;

    println!("{} Project Status", "---".dimmed());

    if let Some(track) = current_track {
        println!("Active Track: {}", track.yellow().bold());
    } else if head_hash.is_some() {
        println!("Active Track: {}", "Detached HEAD".red().bold());
    } else {
        println!("Active Track: {}", "None (Initial)".dimmed());
    }

    if let Some(hash) = head_hash.as_ref() {
        println!("Last Snapshot: {}", hash[..7].cyan());
    }

    // Check for changes
    let indexer = Indexer::new(&repo.object_store);
    let workspace_tree_hash = indexer.index_path(&repo.root_path)?;

    let head_tree_hash = if let Some(hash) = head_hash {
        let (_, content) = repo.object_store.read_object(&hash)?;
        let snapshot_data = Snapshot::deserialize(&content)?;
        Some(snapshot_data.directory_hash)
    } else {
        None
    };

    match (head_tree_hash, workspace_tree_hash) {
        (Some(head), Some(work)) => {
            if head == work {
                println!("{}", "Workspace is clean.".green());
            } else {
                println!("{}", "Uncommitted changes present.".yellow().bold());
                println!("  (Use 'jogen snapshot' to record your work)");
            }
        }
        (None, Some(_)) => {
            println!("{}", "Initial snapshot pending.".yellow().bold());
            println!("  (Use 'jogen snapshot' to record your first state)");
        }
        _ => {
            println!("{}", "Workspace is empty.".dimmed());
        }
    }

    Ok(())
}

pub fn checkout(target: String) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;
    let hydrator = jogen_core::hydrator::Hydrator::new(&repo.object_store);

    // 1. Resolve target to a snapshot hash and determine if it's a track switch
    let (target_snapshot_hash, target_track) = if let Some(hash) = repo.ref_store.resolve_track(&target)? {
        (Some(hash), Some(target.clone()))
    } else {
        // If it's not a track, assume it's a hash
        (Some(target.clone()), None)
    };

    let target_snapshot_hash = match target_snapshot_hash {
        Some(hash) => hash,
        None => return Err(anyhow::anyhow!("Could not resolve target {}", target)),
    };

    println!(
        "{} Checking out {}...",
        "↻".blue(),
        if let Some(ref track) = target_track {
            track.yellow()
        } else {
            target_snapshot_hash[..7].yellow()
        }
    );

    // 2. Safety check: Are there uncommitted changes?
    let current_snapshot_hash = repo.ref_store.read_head()?;

    let head_tree_hash = if let Some(hash) = current_snapshot_hash {
        let (_, content) = repo.object_store.read_object(&hash)?;
        let snapshot_data = Snapshot::deserialize(&content)?;
        Some(snapshot_data.directory_hash)
    } else {
        None
    };

    let indexer = Indexer::new(&repo.object_store);
    let workspace_tree_hash = indexer.index_path(&repo.root_path)?;

    if let (Some(head_tree), Some(workspace_tree)) = (head_tree_hash.as_ref(), workspace_tree_hash.as_ref()) {
        if head_tree != workspace_tree {
            return Err(anyhow::anyhow!(
                "Uncommitted changes found in workspace.\nPlease snapshot or discard changes before checking out."
            ));
        }
    }

    // 3. Resolve target tree
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

    // 4. Apply changes
    if let Some(head_tree) = head_tree_hash.as_ref() {
        hydrator.apply_diff(head_tree, &target_tree_hash, &repo.root_path)?;
    } else {
        // Initial checkout (empty workspace)
        hydrator.hydrate_directory(&target_tree_hash, &repo.root_path)?;
    }

    // 5. Update HEAD
    if let Some(track_name) = target_track {
        repo.ref_store.set_head_to_track(&track_name)?;
    } else {
        repo.ref_store.update_head(&target_snapshot_hash)?;
    }

    println!("{} Checkout complete", "✔".green());

    Ok(())
}

pub fn create_track(name: String, switch: bool) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let current_hash = repo.ref_store.read_head()?;

    if let Some(hash) = current_hash {
        repo.ref_store.create_track(&name, &hash)?;
        println!("{} Created track {}", "✔".green(), name.yellow());
    } else {
        // "Unborn" track - if we are in a fresh repo, we can still create a track
        // by pointing HEAD to it. The first 'snapshot' will then create it.
        println!(
            "{} Creating unborn track {} (will be created on first snapshot)",
            "ℹ".blue(),
            name.yellow()
        );
    }

    if switch {
        repo.ref_store.set_head_to_track(&name)?;
        println!("{} Switched to track {}", "✔".green(), name.yellow());
    }

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
