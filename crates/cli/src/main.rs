mod args;
mod commands;

use anyhow::Result;
use args::{Cli, Commands, ToolSubcommands, TrackSubcommands};
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => {
            commands::actions::handle(args)?;
        }
        Commands::Snapshot(args) => {
            commands::actions::snapshot(args)?;
        }
        Commands::Status => {
            commands::actions::status()?;
        }
        Commands::Log => {
            commands::tools::log()?;
        }
        Commands::Checkout { target } => {
            commands::actions::checkout(target)?;
        }
        Commands::Track(args) => match args.command {
            TrackSubcommands::List => {
                commands::actions::list_tracks()?;
            }
            TrackSubcommands::Create { name, switch } => {
                commands::actions::create_track(name, switch)?;
            }
        },
        Commands::Tools(args) => match args.command {
            ToolSubcommands::Hash { file } => {
                commands::tools::hash_object(file)?;
            }
            ToolSubcommands::Cat { hash } => {
                commands::tools::cat_file(hash)?;
            }
            ToolSubcommands::WriteDir => {
                commands::tools::write_directory()?;
            }
            ToolSubcommands::ReadDir { hash } => {
                commands::tools::read_directory(hash)?;
            }
            ToolSubcommands::WriteSnapshot => {
                commands::tools::write_snapshot()?;
            }
            ToolSubcommands::ReadSnapshot { hash } => {
                commands::tools::read_snapshot(hash)?;
            }
        },
    }

    Ok(())
}
