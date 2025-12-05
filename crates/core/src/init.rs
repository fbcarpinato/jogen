use crate::{JogenError, Result};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
struct CoreConfig {
    version: u8,
}

#[derive(Serialize)]
struct ConfigFile {
    core: CoreConfig,
}

pub fn execute(target_path: Option<PathBuf>) -> Result<PathBuf> {
    let root = target_path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let jogen_dir = root.join(".jogen");

    if jogen_dir.exists() {
        return Err(JogenError::ProjectAlreadyExists(root.display().to_string()));
    }

    let folders = vec![jogen_dir.join("objects")];

    for folder in folders {
        fs::create_dir_all(folder)?;
    }

    let config = ConfigFile {
        core: CoreConfig { version: 1 },
    };
    let config_toml = toml::to_string_pretty(&config)?;
    fs::write(jogen_dir.join("config.toml"), config_toml)?;

    Ok(root)
}
