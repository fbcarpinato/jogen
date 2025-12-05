use crate::args::InitArgs;
use anyhow::Result;
use colored::*;

pub fn handle(args: InitArgs) -> Result<()> {
    let _root = jogen_core::init::execute(args.path)?;

    println!("{}", "Jogen Project Initialized".green().bold());

    Ok(())
}
