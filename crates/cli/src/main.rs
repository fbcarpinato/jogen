mod args;
mod commands;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => {
            commands::init::handle(args)?;
        }
        Commands::HashObject { file } => {
            commands::plumbing::hash_object(file)?;
        }
        Commands::CatFile { hash } => {
            commands::plumbing::cat_file(hash)?;
        }
    }

    Ok(())
}
