use clap::{Args, Parser, Subcommand};
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
}

#[derive(Args)]
pub struct InitArgs {
    pub path: Option<PathBuf>,
}
