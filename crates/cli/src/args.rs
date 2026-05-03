use clap::{Args, Parser, Subcommand};
use jogen_core::objects::snapshot::SnapshotContext;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "jogen")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Jogen project
    Init(InitArgs),

    /// Take a snapshot of the current workspace
    Snapshot(SnapshotArgs),

    /// Show the current state of the workspace
    Status,

    /// Show the snapshot log
    Log {
        /// Show full history graph (all parents) instead of linear first-parent view
        #[arg(short, long)]
        expand: bool,
    },

    /// Restore the workspace to a specific snapshot or track
    Checkout { target: String },

    /// Semantically compare a file with its incoming version during a conflict
    Diff { file: PathBuf },

    /// Integrate a track into the current track
    Integrate(IntegrateArgs),

    /// Manage tracks (branches)
    Track(TrackArgs),

    /// Low-level plumbing tools
    Tools(ToolArgs),
}

#[derive(Args)]
pub struct InitArgs {
    pub path: Option<PathBuf>,
}

#[derive(Args)]
pub struct IntegrateArgs {
    /// The target track or snapshot to integrate
    pub target: Option<String>,

    /// Continue integration after resolving conflicts
    #[arg(long)]
    pub r#continue: bool,

    /// Abort the current integration and return to previous state
    #[arg(long)]
    pub abort: bool,
}

#[derive(Args)]
pub struct SnapshotArgs {
    /// Description of the changes
    #[arg(short, long)]
    pub message: String,

    /// The intent of these changes
    #[arg(short, long, value_enum)]
    pub context: SnapshotContext,
}

#[derive(Args)]
pub struct TrackArgs {
    #[command(subcommand)]
    pub command: TrackSubcommands,
}

#[derive(Subcommand)]
pub enum TrackSubcommands {
    /// List all tracks
    List,
    /// Create a new track
    Create {
        name: String,
        /// Switch to the new track immediately
        #[arg(short, long)]
        switch: bool,
    },
}

#[derive(Args)]
pub struct ToolArgs {
    #[command(subcommand)]
    pub command: ToolSubcommands,
}

#[derive(Subcommand)]
pub enum ToolSubcommands {
    /// Calculate hash and write blob from file
    Hash { file: PathBuf },
    /// Provide content or type and size information for repository objects
    Cat { hash: String },
    /// Create a tree object from the current directory
    WriteDir,
    /// Read a directory object
    ReadDir { hash: String },
    /// Create a snapshot object
    WriteSnapshot,
    /// Read a snapshot object
    ReadSnapshot { hash: String },
}
