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
    Init(InitArgs),

    HashObject { file: PathBuf },

    CatFile { hash: String },

    WriteDirectory {},

    ReadDirectory { hash: String },

    WriteSnapshot {},

    ReadSnapshot { hash: String },

    Save(SaveArgs),

    History {},

    Checkout { hash: String },

    CreateTrack { name: String },

    ListTracks {},
}

#[derive(Args)]
pub struct InitArgs {
    pub path: Option<PathBuf>,
}

#[derive(Args)]
pub struct SaveArgs {
    #[arg(short, long)]
    pub message: String,

    #[arg(short, long, value_enum)]
    pub context: SnapshotContext,
}
