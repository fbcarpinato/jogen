use anyhow::Result;
use chrono::Utc;
use colored::*;

use crate::{
    args::{InitArgs, IntegrateArgs, SnapshotArgs},
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
    let indexer = Indexer::new(&repo.object_store, &repo.root_path);
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

    if let Ok(Some(status)) = repo.ref_store.get_integration_status() {
        println!("\n{} {}", "Status:".red().bold(), "INTEGRATING".red());
        println!(
            "Integrating target: {} ({})",
            status.target_name.yellow(),
            status.target_hash[..7].cyan()
        );
    }

    // Check for changes
    let indexer = Indexer::new(&repo.object_store, &repo.root_path);
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

    let indexer = Indexer::new(&repo.object_store, &repo.root_path);
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

pub fn integrate(args: IntegrateArgs) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;
    let hydrator = jogen_core::hydrator::Hydrator::new(&repo.object_store);

    // --- HANDLE ABORT ---
    if args.abort {
        let integration_status = repo.ref_store.get_integration_status()?;
        if integration_status.is_none() {
            println!("{} Not currently integrating.", "ℹ".blue());
            return Ok(());
        }

        let integration_status = integration_status.unwrap();
        println!("{} Aborting integration...", "⚠".yellow());

        // Delete only integration conflict markers tracked for this integration.
        for conflict_path in &integration_status.conflict_paths {
            let absolute = repo.root_path.join(conflict_path);
            if absolute.exists() {
                std::fs::remove_file(absolute)?;
            }
        }

        // Restore HEAD state to wipe partial hydration and remove stale files.
        if let Some(head_hash) = repo.ref_store.read_head()? {
            let (_, content) = repo.object_store.read_object(&head_hash)?;
            let snapshot = Snapshot::deserialize(&content)?;
            let indexer = Indexer::new(&repo.object_store, &repo.root_path);
            if let Some(current_tree_hash) = indexer.index_path(&repo.root_path)? {
                hydrator.apply_diff(&current_tree_hash, &snapshot.directory_hash, &repo.root_path)?;
            } else {
                hydrator.hydrate_directory(&snapshot.directory_hash, &repo.root_path)?;
            }
        }
        repo.ref_store.clear_integration()?;

        println!("{} Integration aborted. Workspace restored.", "✔".green());
        return Ok(());
    }

    let current_track_opt = repo.ref_store.current_track()?;
    let current_track = current_track_opt.ok_or_else(|| anyhow::anyhow!("You must be on a track to integrate."))?;
    let head_hash = repo.ref_store.read_head()?.ok_or_else(|| anyhow::anyhow!("Head is empty. Cannot integrate."))?;

    // --- HANDLE CONTINUE ---
    if args.r#continue {
        let integration_status = repo.ref_store.get_integration_status()?
            .ok_or_else(|| anyhow::anyhow!("No integration in progress."))?;

        // Check only the conflict markers recorded for this integration.
        let mut conflicts_remain = false;
        for conflict_path in &integration_status.conflict_paths {
            let absolute = repo.root_path.join(conflict_path);
            if absolute.exists() {
                println!("  - {}", conflict_path.red());
                conflicts_remain = true;
            }
        }

        if conflicts_remain {
            return Err(anyhow::anyhow!("Cannot continue. Unresolved '.incoming' files still exist."));
        }

        println!("{} Finalizing integration...", "⚙".blue());

        // Snapshot the resolved state
        let indexer = Indexer::new(&repo.object_store, &repo.root_path);
        let resolved_tree_hash = indexer.index_path(&repo.root_path)?.ok_or_else(|| anyhow::anyhow!("Workspace is empty"))?;

        let parent_hashes = vec![head_hash, integration_status.target_hash];
        let message = format!("Merge track '{}' into '{}'", integration_status.target_name, current_track);
        
        let snapshot = Snapshot::new(
            resolved_tree_hash,
            parent_hashes,
            "Jogen User <user@jogen.com>".to_string(),
            chrono::Utc::now().timestamp(),
            jogen_core::objects::snapshot::SnapshotContext::Merge,
            message,
        );

        let snapshot_hash = repo.object_store.write_object(snapshot.serialize()?.as_ref(), ObjectType::Snapshot)?;
        repo.ref_store.update_head(&snapshot_hash)?;
        repo.ref_store.clear_integration()?;

        println!("{} Integration complete. Created merge snapshot {}", "✔".green(), snapshot_hash[..7].yellow());
        return Ok(());
    }

    // --- HANDLE NEW INTEGRATION ---
    if repo.ref_store.get_integration_status()?.is_some() {
        return Err(anyhow::anyhow!("An integration is already in progress. Use --continue or --abort."));
    }

    let target = args.target.ok_or_else(|| anyhow::anyhow!("Must provide a target to integrate."))?;
    let target_hash = repo.ref_store.resolve_track(&target)?.ok_or_else(|| anyhow::anyhow!("Could not resolve target track: {}", target))?;

    if head_hash == target_hash {
        println!("{} Already up to date.", "✔".green());
        return Ok(());
    }

    println!("{} Integrating {} into {}...", "⚙".blue(), target.yellow(), current_track.yellow());

    let graph = jogen_core::graph::GraphTraversal::new(&repo.object_store);
    let base_hash = graph.find_common_ancestor(&head_hash, &target_hash)?;

    let get_tree = |hash: &str| -> Result<String> {
        let (kind, content) = repo.object_store.read_object(hash)?;
        if kind == ObjectType::Snapshot {
            let snapshot = Snapshot::deserialize(&content)?;
            Ok(snapshot.directory_hash)
        } else {
            Err(anyhow::anyhow!("Expected snapshot, got {}", kind))
        }
    };

    let base_tree = match base_hash {
        Some(ref h) => Some(get_tree(h)?),
        None => None,
    };
    let head_tree = get_tree(&head_hash)?;
    let target_tree = get_tree(&target_hash)?;

    let merge_engine = jogen_core::merge::MergeEngine::new(&repo.object_store);
    let merge_result = merge_engine.merge_trees(base_tree.as_deref(), Some(&head_tree), Some(&target_tree), "");
    let merged_tree_hash = merge_result
        .tree_hash
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Merge failed to produce a tree hash."))?;
    hydrator.apply_diff(&head_tree, merged_tree_hash, &repo.root_path)?;

    if !merge_result.conflicts.is_empty() {
        println!("{} Conflicts found! Pausing integration.", "⚠".yellow().bold());
        
        let conflict_paths = hydrator.write_conflict_files(&merge_result.conflicts, &repo.root_path)?;
        repo.ref_store.begin_integration(
            base_hash.as_deref().unwrap_or(""),
            &target_hash,
            &target,
            &conflict_paths,
        )?;

        println!("\nThe following files have conflicts. Incoming versions have been saved alongside your files:");
        for conflict in &merge_result.conflicts {
            println!("  - {}", conflict.path.red());
        }
        println!("\nTo resolve:");
        println!("  1. Run 'jogen diff <file>' to semantically compare changes.");
        println!("  2. Edit your file to the desired final state.");
        println!("  3. Delete the .incoming file.");
        println!("  4. Run 'jogen integrate --continue'.");
        
        return Err(anyhow::anyhow!("Integration paused due to conflicts."));
    }

    // No conflicts, auto-commit
    let parent_hashes = vec![head_hash, target_hash];
    let message = format!("Merge track '{}' into '{}'", target, current_track);
    let snapshot = Snapshot::new(
        merged_tree_hash.to_string(),
        parent_hashes,
        "Jogen User <user@jogen.com>".to_string(),
        chrono::Utc::now().timestamp(),
        jogen_core::objects::snapshot::SnapshotContext::Merge,
        message,
    );

    let snapshot_hash = repo.object_store.write_object(snapshot.serialize()?.as_ref(), ObjectType::Snapshot)?;
    repo.ref_store.update_head(&snapshot_hash)?;

    println!("{} Integration complete. Created merge snapshot {}", "✔".green(), snapshot_hash[..7].yellow());
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

pub fn diff(file: std::path::PathBuf) -> Result<()> {
    let repo = JogenRepo::from_cwd()?;

    let mut incoming_file = file.clone();
    if let Some(ext) = incoming_file.extension() {
        let new_ext = format!("incoming.{}", ext.to_string_lossy());
        incoming_file.set_extension(new_ext);
    } else {
        incoming_file.set_extension("incoming");
    }

    if !incoming_file.exists() {
        return Err(anyhow::anyhow!("No conflict found for this file. Expected '{}' to exist.", incoming_file.display()));
    }

    let head_content = std::fs::read(&file)?;
    let target_content = std::fs::read(&incoming_file)?;

    let mut base_content = Vec::new();
    if let Ok(Some(status)) = repo.ref_store.get_integration_status() {
        if !status.base_hash.is_empty() {
             // Find base tree
             if let Ok((kind, content)) = repo.object_store.read_object(&status.base_hash) {
                 if kind == ObjectType::Snapshot {
                     if let Ok(snapshot) = Snapshot::deserialize(&content) {
                          // Note: extracting the exact file from a tree requires a full tree walk.
                          // For simplicity in this demo block-level merge, if we can't easily fetch it,
                          // we might skip. But let's assume we can fetch it if we had a full tree walker.
                          // As a shortcut, we might just try to find the blob hash via hydrator, or
                          // keep it empty for now if it's too complex.
                          // Since finding the exact blob hash for a file requires traversing the Directory objects,
                          // let's do a quick traversal.
                          if let Ok(blob_hash) = find_blob_in_tree(&repo.object_store, &snapshot.directory_hash, &file, &repo.root_path) {
                              if let Ok((_, blob_data)) = repo.object_store.read_object(&blob_hash) {
                                  base_content = blob_data;
                              }
                          }
                     }
                 }
             }
        }
    }

    let semantic_engine = jogen_core::semantic::SemanticEngine::new();

    let head_parsed = semantic_engine.parse_file(&file, &head_content);
    let target_parsed = semantic_engine.parse_file(&incoming_file, &target_content);
    let base_parsed = if !base_content.is_empty() { semantic_engine.parse_file(&file, &base_content) } else { None };

    if let (Some((_, head_tree)), Some((_, target_tree))) = (head_parsed, target_parsed) {
        println!("{} Semantic Diff: {}", "★".purple(), file.display().to_string().bold());
        println!("{}", "=".repeat(40).dimmed());

        let head_blocks = semantic_engine.extract_blocks(&head_tree, &head_content);
        let target_blocks = semantic_engine.extract_blocks(&target_tree, &target_content);
        
        let base_blocks = if let Some((_, base_tree)) = base_parsed {
            semantic_engine.extract_blocks(&base_tree, &base_content)
        } else {
            Vec::new()
        };

        let mut head_map = std::collections::HashMap::new();
        for block in head_blocks {
            head_map.insert(format!("{}:{}", block.kind, block.name), block);
        }

        let mut target_map = std::collections::HashMap::new();
        for block in target_blocks {
            target_map.insert(format!("{}:{}", block.kind, block.name), block);
        }
        
        let mut base_map = std::collections::HashMap::new();
        for block in base_blocks {
            base_map.insert(format!("{}:{}", block.kind, block.name), block);
        }

        let all_keys: std::collections::HashSet<_> = head_map.keys().chain(target_map.keys()).cloned().collect();
        let mut sorted_keys: Vec<_> = all_keys.into_iter().collect();
        sorted_keys.sort();

        let mut found_changes = false;

        for key in sorted_keys {
            let head_block = head_map.get(&key);
            let target_block = target_map.get(&key);
            let base_block = base_map.get(&key);

            let print_header = |prefix: colored::ColoredString, block: &jogen_core::semantic::SemanticBlock| {
                let crumbs = if block.breadcrumbs.len() > 1 {
                    format!(" [{}] ", block.breadcrumbs[..block.breadcrumbs.len()-1].join(" > ").dimmed())
                } else {
                    " ".to_string()
                };
                println!("{} {}{}'{}' (Lines {}-{})", prefix, block.kind, crumbs, block.name.bold(), block.start_line, block.end_line);
            };

            match (head_block, target_block) {
                (Some(h), Some(t)) => {
                    if h.content != t.content {
                        print_header("Mod".yellow().bold(), h);
                        println!("    {} Your version differs from the incoming version.", "→".dimmed());
                        
                        if let Some(b) = base_block {
                             println!("    {} Original Base version:", "→".dimmed());
                             for line in b.content.lines() {
                                 println!("      {}", line.dimmed());
                             }
                        }

                        println!("    {} Changes (Yours vs Incoming):", "→".dimmed());
                        let diff = similar::TextDiff::from_lines(&h.content, &t.content);
                        for change in diff.iter_all_changes() {
                            let sign = match change.tag() {
                                similar::ChangeTag::Delete => "-".red(),
                                similar::ChangeTag::Insert => "+".green(),
                                similar::ChangeTag::Equal => " ".dimmed(),
                            };
                            let mut line_str = change.value().to_string();
                            if line_str.ends_with('\n') {
                                line_str.pop();
                            }
                            if line_str.ends_with('\r') {
                                line_str.pop();
                            }
                            match change.tag() {
                                similar::ChangeTag::Delete => println!("      {} {}", sign, line_str.red().dimmed()),
                                similar::ChangeTag::Insert => println!("      {} {}", sign, line_str.green().dimmed()),
                                similar::ChangeTag::Equal => println!("      {} {}", sign, line_str.dimmed()),
                            }
                        }
                        
                        println!();
                        found_changes = true;
                    }
                }
                (Some(h), None) => {
                    print_header("Del".red().bold(), h);
                    println!("    {} This block was removed in the incoming track.", "→".dimmed());
                    println!("    {} Removed code:", "→".dimmed());
                    for line in h.content.lines() {
                        println!("      {}", line.red().dimmed());
                    }
                    println!();
                    found_changes = true;
                }
                (None, Some(t)) => {
                    print_header("Add".green().bold(), t);
                    println!("    {} This block was added in the incoming track.", "→".dimmed());
                    println!("    {} Added code:", "→".dimmed());
                    for line in t.content.lines() {
                        println!("      {}", line.green().dimmed());
                    }
                    println!();
                    found_changes = true;
                }
                (None, None) => {}
            }
        }

        if !found_changes {
            println!("No top-level structural changes detected.");
        }
        
        println!("{}", "=".repeat(40).dimmed());
        println!("Please edit {} to resolve the logic, then delete {}.", file.display().to_string().yellow(), incoming_file.display().to_string().red());

    } else {
        println!("{} Unsupported language for Semantic Diff.", "⚠".yellow());
        println!("Please manually compare '{}' and '{}' in your editor.", file.display(), incoming_file.display());
    }

    Ok(())
}

fn find_blob_in_tree(store: &jogen_core::object_store::ObjectStore, tree_hash: &str, target_path: &std::path::Path, root_path: &std::path::Path) -> Result<String> {
    let relative_path = target_path.strip_prefix(root_path).unwrap_or(target_path);
    let components: Vec<_> = relative_path.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    
    let mut current_hash = tree_hash.to_string();
    
    for (i, component) in components.iter().enumerate() {
        let (kind, content) = store.read_object(&current_hash)?;
        if kind != ObjectType::Directory {
            return Err(anyhow::anyhow!("Expected directory"));
        }
        let dir = jogen_core::objects::directory::Directory::parse(&content)?;
        
        let mut found = false;
        for entry in dir.entries() {
            if &entry.name == component {
                current_hash = entry.hash.clone();
                found = true;
                break;
            }
        }
        
        if !found {
            return Err(anyhow::anyhow!("Path not found in tree"));
        }
        
        // If it's the last component, we expect a blob
        if i == components.len() - 1 {
            return Ok(current_hash);
        }
    }
    
    Err(anyhow::anyhow!("Not a file"))
}
