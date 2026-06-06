# How Jogen Works

Jogen is an experimental version control system. It uses the same broad idea as Git: content-addressed objects form a graph of immutable history. The vocabulary and implementation are intentionally different.

Jogen currently focuses on the local repository model:

- Store file contents as immutable objects.
- Store directory trees as immutable objects.
- Store snapshots as immutable objects with parent links.
- Move named tracks through history as new snapshots are created.
- Restore the workspace from a snapshot.
- Integrate one track into another with a three-way merge.
- Use semantic diffing to help resolve code conflicts.

## Concepts

### Project

A Jogen project is any directory containing a `.jogen` folder. Commands search upward from the current directory until they find `.jogen`, so you can run commands from nested folders inside a project.

Running `jogen init` creates:

```text
.jogen/
  HEAD
  config.toml
  objects/
  refs/
    tracks/
```

`config.toml` currently stores the core repository format version:

```toml
[core]
version = 1
```

`HEAD` is initialized as:

```text
ref: refs/tracks/main
```

That means the project starts on the `main` track, but the track file itself is not created until the first snapshot updates it.

### Object Store

Objects live in `.jogen/objects`. Every object is addressed by a BLAKE3 hash of its object header plus its serialized data. The hash determines the storage path:

```text
.jogen/objects/<first-two-hash-chars>/<remaining-hash-chars>
```

For example, an object with hash `abcdef...` is stored under:

```text
.jogen/objects/ab/cdef...
```

Object files are compressed with Zstandard. Before compression, each object contains a fixed 10-byte header followed by the object payload.

Header layout:

| Bytes | Field | Meaning |
| --- | --- | --- |
| 0 | version | Object store version, currently `1` |
| 1 | kind | `1` blob, `2` directory, `3` snapshot |
| 2..10 | size | Payload size as little-endian `u64` |

Because the hash includes the header and payload, two objects with the same payload but different object types produce different hashes.

### Blob Objects

A blob stores raw file bytes. The blob payload is exactly the file content.

Jogen does not interpret blob content when writing a snapshot. Text files, binary files, generated files, and source files are all stored as bytes unless ignored by `.jogenignore` or skipped by the indexer.

### Directory Objects

A directory stores sorted entries for files and child directories. This is similar to a Git tree.

Each directory entry serializes as:

```text
<mode> <name>\0<32-byte hash>
```

Supported serialized modes are:

| Mode | Meaning |
| --- | --- |
| `100644` | regular file |
| `100755` | executable file |
| `040000` | directory |

The hash is stored as 32 raw bytes and decoded back to hex when read.

Directory entries are sorted by name before serialization. This makes directory hashes stable regardless of filesystem traversal order.

Current implementation note: the object format supports executable entries, and checkout can restore executable permissions on Unix, but the current indexer records filesystem files as regular files.

### Snapshot Objects

A snapshot is Jogen's equivalent of a Git commit. It points at a root directory object and records metadata about why the change exists.

Snapshot payload format:

```text
directory <directory-hash>
parent <parent-snapshot-hash>
parent <second-parent-snapshot-hash>
author <author>
time <unix-timestamp>
context <context>

<message>
```

Root snapshots have no `parent` lines. Normal snapshots have one parent. Integration snapshots have two parents: the current track head and the integrated target head.

Supported contexts are:

| Context | Intended use |
| --- | --- |
| `initial` | first project state |
| `feature` | new behavior |
| `fix` | bug fix |
| `refactor` | behavior-preserving restructure |
| `docs` | documentation |
| `chore` | maintenance |
| `merge` | integration snapshot |

Every regular snapshot requires a context and message:

```sh
jogen snapshot --context feature --message "Add search endpoint"
```

The current CLI writes a placeholder author of `Jogen User <user@jogen.com>`.

### Tracks

A track is Jogen's branch equivalent. Tracks are files under `.jogen/refs/tracks` whose contents are snapshot hashes.

For example:

```text
.jogen/refs/tracks/main
.jogen/refs/tracks/feature-login
```

`HEAD` can point at a track:

```text
ref: refs/tracks/main
```

Or it can point directly at a snapshot hash. A direct hash means the project is in a detached HEAD state.

When `HEAD` points at a track, `jogen snapshot` advances that track. When `HEAD` is detached, `jogen snapshot` updates `HEAD` directly.

## Indexing

Indexing is the process of turning the working directory into a root directory object.

Jogen recursively walks the project root and writes objects as it goes:

1. Skip `.jogen` entirely.
2. Load ignore rules from `.jogenignore` if present.
3. For each regular file, read the bytes and write a blob object.
4. For each directory, write child entries and then write a directory object.
5. Return the root directory hash.

There is no staging area. A snapshot records the whole current workspace, minus ignored paths.

If a project contains no indexable files, snapshot creation fails with `Nothing to snapshot` or `Cannot snapshot an empty project`, depending on the command.

## Command Reference

### `jogen init [path]`

Creates a `.jogen` repository in the current directory or in `path`.

Example:

```sh
jogen init
```

What it does:

- Creates `.jogen/objects`.
- Creates `.jogen/refs/tracks`.
- Writes `.jogen/config.toml`.
- Writes `.jogen/HEAD` pointing at `refs/tracks/main`.

It refuses to initialize if `.jogen` already exists at the target path.

### `jogen status`

Shows the current track, last snapshot, integration state, and whether the workspace matches `HEAD`.

Status compares the current workspace tree hash to the directory hash stored in the current snapshot.

Possible workspace states include:

- Clean workspace: current tree hash equals the `HEAD` snapshot tree hash.
- Uncommitted changes: current tree hash differs from the `HEAD` snapshot tree hash.
- Initial snapshot pending: the workspace has files but `HEAD` does not resolve to a snapshot yet.
- Empty workspace: there are no indexable files.
- Integrating: `.jogen/INTEGRATING` exists.

### `jogen snapshot --context <context> --message <message>`

Records the current workspace as a new snapshot.

Example:

```sh
jogen snapshot --context fix --message "Handle empty input"
```

What it does:

1. Indexes the workspace into blobs and directories.
2. Reads the current `HEAD` snapshot, if any.
3. Creates a snapshot object whose parent is the current `HEAD` snapshot.
4. Writes the snapshot object to `.jogen/objects`.
5. Updates `HEAD`, or the current track if `HEAD` points at a track.

Short options are also available:

```sh
jogen snapshot -c docs -m "Document storage format"
```

### `jogen log [--expand]`

Prints snapshot history from `HEAD`.

Without `--expand`, Jogen follows only the first parent of each snapshot. This is similar to a first-parent Git log.

```sh
jogen log
```

With `--expand`, Jogen traverses all parents from `HEAD`, including both parents of integration snapshots.

```sh
jogen log --expand
```

### `jogen track list`

Lists known tracks and marks the active one.

```sh
jogen track list
```

Tracks are sorted by name.

### `jogen track create <name> [--switch]`

Creates a new track pointing at the current `HEAD` snapshot.

```sh
jogen track create feature-search
```

Create and switch immediately:

```sh
jogen track create feature-search --switch
```

If the repository has no snapshots yet, Jogen treats the track as unborn. With `--switch`, `HEAD` is pointed at the new track name and the first snapshot will create the track file.

### `jogen checkout <target>`

Restores the workspace to a track or snapshot.

```sh
jogen checkout main
```

```sh
jogen checkout <snapshot-hash>
```

Resolution rules:

- If `<target>` matches a track name, Jogen checks out that track and makes `HEAD` symbolic.
- Otherwise, Jogen treats `<target>` as a snapshot hash and enters detached HEAD state.

Before changing files, checkout checks whether the current workspace differs from `HEAD`. If there are uncommitted changes, checkout fails and asks you to snapshot or discard them first.

Checkout does not delete and rewrite the whole workspace. It applies a tree diff from the current snapshot directory to the target snapshot directory:

- Unchanged files are left alone.
- Changed files are overwritten.
- New files are created.
- Removed files are deleted.
- Changed directories are updated recursively.

### `jogen integrate <target>`

Integrates another track into the current track. This is Jogen's merge operation.

```sh
jogen integrate feature-search
```

Current requirements:

- You must be on a track, not detached HEAD.
- `HEAD` must resolve to a snapshot.
- `<target>` must currently resolve to a track name.

What it does:

1. Resolves the current track head and target track head.
2. Finds a common ancestor snapshot.
3. Reads the base, current, and target directory trees.
4. Performs a three-way tree merge.
5. Applies the merged tree to the workspace.
6. If there are no conflicts, creates a `merge` snapshot with two parents.
7. If there are conflicts, writes incoming conflict files and pauses integration.

The merge engine uses standard three-way rules:

- If current and target agree, use that version.
- If only target changed relative to base, use target.
- If only current changed relative to base, use current.
- If both changed a directory, merge inside it recursively.
- If both changed a text file, attempt an automatic text merge.
- If automatic merge fails, keep the current version and write the incoming version beside it.

### `jogen integrate --continue`

Finishes a paused integration after you resolve conflicts.

```sh
jogen integrate --continue
```

Jogen checks the conflict marker paths recorded in `.jogen/INTEGRATING`. If any `.incoming` files still exist, continue fails. If all markers are gone, Jogen snapshots the resolved workspace as a `merge` snapshot with two parents and clears the integration state.

### `jogen integrate --abort`

Cancels a paused integration.

```sh
jogen integrate --abort
```

Abort removes recorded incoming conflict files, restores the workspace back to the current `HEAD` snapshot, and deletes `.jogen/INTEGRATING`.

### `jogen diff <file>`

Shows a semantic comparison between your file and its incoming conflict version.

```sh
jogen diff src/main.rs
```

During a conflicted integration, Jogen writes incoming versions next to the conflicted files:

| Original | Incoming marker |
| --- | --- |
| `src/main.rs` | `src/main.incoming.rs` |
| `README` | `README.incoming` |

`jogen diff <file>` reads both files and, when possible, parses them with tree-sitter. Supported extensions are:

| Language | Extensions |
| --- | --- |
| Rust | `.rs` |
| JavaScript/TypeScript | `.js`, `.ts`, `.jsx`, `.tsx` |
| Python | `.py` |

For supported languages, the diff groups changes by structural blocks such as functions, classes, structs, impl blocks, declarations, imports, exports, constants, and statics.

If the language is unsupported, Jogen tells you to compare the original and incoming files manually.

Conflict resolution flow:

```sh
jogen diff path/to/file.rs
```

Then:

- Edit the original file to the final desired result.
- Delete the corresponding `.incoming` file.
- Repeat for every conflicted file.
- Run `jogen integrate --continue`.

## Plumbing Tools

The `tools` subcommands expose lower-level object operations. They are useful for debugging the repository format.

### `jogen tools hash <file>`

Reads a file, writes it as a blob object, and prints the object hash.

```sh
jogen tools hash README.md
```

### `jogen tools cat <hash>`

Reads an object and writes its payload to stdout. The object type is printed to stderr.

```sh
jogen tools cat <hash>
```

For blobs, stdout is the original file content. For directories and snapshots, stdout is the serialized payload.

### `jogen tools write-dir`

Indexes the current project root and prints the resulting root directory hash.

```sh
jogen tools write-dir
```

This writes any missing blob and directory objects, but it does not create a snapshot or update `HEAD`.

### `jogen tools read-dir <hash>`

Reads a directory object and prints its entries.

```sh
jogen tools read-dir <directory-hash>
```

### `jogen tools write-snapshot`

Creates a snapshot object from the current workspace and prints the snapshot hash.

```sh
jogen tools write-snapshot
```

This plumbing command creates a snapshot object with no parents and does not update `HEAD`. For normal usage, prefer `jogen snapshot`.

### `jogen tools read-snapshot <hash>`

Reads a snapshot object and prints its metadata.

```sh
jogen tools read-snapshot <snapshot-hash>
```

## Ignore Rules

Jogen reads ignore patterns from `.jogenignore` at the project root. The syntax is handled by the `ignore` crate's gitignore parser, so it follows gitignore-style matching.

`.jogen` is always skipped, even if it is not listed in `.jogenignore`.

There is currently no automatic use of `.gitignore`; add patterns to `.jogenignore` if you want Jogen to skip build outputs or generated files.

Example:

```gitignore
target/
node_modules/
*.log
```

## History Graph

Jogen history is a graph of snapshots.

Root snapshot:

```text
A
```

Linear history:

```text
A <- B <- C
```

Track divergence:

```text
A <- B <- C   main
      \
       D <- E feature
```

Integration snapshot:

```text
A <- B <- C <- M   main
      \      /
       D <- E     feature
```

`M` has two parents: `C` and `E`. `jogen log` follows `C` as the first parent. `jogen log --expand` traverses both `C` and `E`.

## Checkout And Workspace Restoration

Jogen restores files by comparing two directory trees and applying the minimal set of filesystem operations it can infer from those trees.

When applying a diff from old tree to new tree:

- If an entry exists in both trees with the same mode and hash, it is unchanged.
- If an entry exists in both trees but differs, files are overwritten and directories are recursively updated.
- If an entry exists only in the new tree, it is created.
- If an entry exists only in the old tree, it is removed.

This means checkout depends on the current workspace matching `HEAD`. If the workspace has unsnapped changes, Jogen refuses to check out another target rather than overwriting work.

## Integration State

When an integration pauses for conflicts, Jogen writes `.jogen/INTEGRATING`.

The file stores:

```text
<base-snapshot-hash>
<target-snapshot-hash>
<target-name>
conflicts <count>
<incoming-marker-path>
<incoming-marker-path>
...
```

This lets `status`, `integrate --continue`, and `integrate --abort` know that an integration is in progress and which incoming marker files belong to it.

## Differences From Git

Jogen is Git-like, but it is not Git.

Important differences:

- There is no staging area; snapshots record the whole indexable workspace.
- There is no rebase or history rewrite command.
- There are no remotes or network synchronization commands.
- There are no tags.
- There is no packed object storage.
- There is no user identity configuration yet; author metadata is currently fixed.
- Tracks are local files under `.jogen/refs/tracks`.
- Conflict markers are separate `.incoming` files rather than inline conflict markers.
- Semantic diff is available for supported code files during conflict resolution.

## Current Limitations

Jogen is under active development. Current limitations include:

- The CLI binary is currently built as `jogen-cli`; examples use `jogen` as the intended command name.
- Snapshot author metadata is hard-coded.
- There is no staging area or partial snapshot support.
- `integrate <target>` resolves targets as track names, not arbitrary snapshot hashes.
- `.jogenignore` is supported, but `.gitignore` is not automatically imported.
- The object store is local-only.
- The repository format may change while the project is experimental.

## Typical Workflow

```sh
jogen init
jogen snapshot -c initial -m "Initial snapshot"

jogen track create feature-api --switch
# edit files
jogen status
jogen snapshot -c feature -m "Add API endpoint"

jogen checkout main
jogen integrate feature-api
jogen log --expand
```

If integration conflicts:

```sh
jogen diff src/example.rs
# edit src/example.rs
rm src/example.incoming.rs
jogen integrate --continue
```

Or cancel it:

```sh
jogen integrate --abort
```
