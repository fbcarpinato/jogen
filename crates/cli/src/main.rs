mod args;
mod commands;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => {
            commands::actions::handle(args)?;
        }
        Commands::HashObject { file } => {
            commands::tools::hash_object(file)?;
        }
        Commands::CatFile { hash } => {
            commands::tools::cat_file(hash)?;
        }
        Commands::WriteDirectory {} => {
            commands::tools::write_directory()?;
        }
        Commands::ReadDirectory { hash } => {
            commands::tools::read_directory(hash)?;
        }
        Commands::WriteSnapshot {} => {
            commands::tools::write_snapshot()?;
        }
        Commands::ReadSnapshot { hash } => {
            commands::tools::read_snapshot(hash)?;
        }
        Commands::Save(args) => {
            commands::actions::save(args)?;
        }
        Commands::History {} => {
            commands::tools::history()?;
        }
        Commands::Checkout { hash } => {
            commands::actions::checkout(hash)?;
        }
        Commands::CreateTrack { name } => {
            commands::actions::create_track(name)?;
        }
        Commands::ListTracks {} => {
            commands::tools::list_tracks()?;
        }
    }

    Ok(())
}
