# Jogen

**The Atomic Version Control System.**

Jogen is an experimental version control system written in Rust. It is my take on Git: a content-addressed object database, snapshots, tracks, checkout, history traversal, and integration, with a stronger emphasis on explicit development intent.

> 🚧 **Status:** Active Development

## What Jogen Does

Jogen records your workspace as immutable snapshots. Each snapshot points to a directory tree, zero or more parent snapshots, an author, a timestamp, a required context type, and a message.

At a high level:

- Files become **blob** objects.
- Folders become **directory** objects that point to blobs and child directories.
- Commits become **snapshot** objects that point to a root directory and parent snapshots.
- Branches become **tracks** stored under `.jogen/refs/tracks`.
- `HEAD` either points at a track or directly at a snapshot.

The closest Git equivalents are:

| Jogen | Git equivalent |
| --- | --- |
| snapshot | commit |
| track | branch |
| integrate | merge |
| directory | tree |
| blob | blob |
| `.jogen` | `.git` |

Jogen is not Git-compatible and does not currently implement remotes, staging, rebasing, tags, authentication, or packed storage.

## Install And Run

Build the CLI from the workspace:

```sh
cargo build -p jogen-cli
```

Run it directly with Cargo:

```sh
cargo run -p jogen-cli -- --help
```

Or use the compiled binary:

```sh
./target/debug/jogen-cli --help
```

The examples below use `jogen` as the command name. If you have not installed or aliased the binary, replace `jogen` with `cargo run -p jogen-cli --` or `./target/debug/jogen-cli`.

## Quick Start

Initialize a project:

```sh
jogen init
```

Create the first snapshot:

```sh
jogen snapshot --context initial --message "Initial snapshot"
```

Check workspace state:

```sh
jogen status
```

Create and switch to a new track:

```sh
jogen track create feature-login --switch
```

Record work on that track:

```sh
jogen snapshot --context feature --message "Add login flow"
```

Switch back to `main`:

```sh
jogen checkout main
```

Integrate the feature track:

```sh
jogen integrate feature-login
```

View history:

```sh
jogen log
```

View the full parent graph, including merge parents:

```sh
jogen log --expand
```

## Documentation

Read our design specs in [`docs/`](docs/):

- [**Philosophy**](docs/01_PHILOSOPHY.md) - The why.
- [**How Jogen Works**](docs/02_HOW_IT_WORKS.md) - The repository format, object model, commands, and workflows.

## Contributing

Contributions are welcome! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Jogen is licensed under the [MIT License](LICENSE).
