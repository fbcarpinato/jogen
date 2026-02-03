use crate::{JogenError, Result};
use std::{fs, path::PathBuf};

pub struct RefStore {
    root_path: PathBuf,
}

impl RefStore {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    pub fn read_head(&self) -> Result<Option<String>> {
        let head_path = self.root_path.join(".jogen/HEAD");

        if !head_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&head_path).map_err(JogenError::Io)?;
        let content = content.trim();

        if let Some(ref_path) = content.strip_prefix("ref: ") {
            self.read_ref(ref_path)
        } else {
            Ok(Some(content.to_string()))
        }
    }

    fn update_ref(&self, ref_name: &str, hash: &str) -> Result<()> {
        let path = self.root_path.join(".jogen").join(ref_name);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(JogenError::Io)?;
        }

        fs::write(path, hash).map_err(JogenError::Io)?;
        Ok(())
    }

    pub fn read_ref(&self, ref_name: &str) -> Result<Option<String>> {
        let path = self.root_path.join(".jogen").join(ref_name);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path).map_err(JogenError::Io)?;
        Ok(Some(content.trim().to_string()))
    }

    pub fn update_head(&self, new_hash: &str) -> Result<()> {
        let head_path = self.root_path.join(".jogen/HEAD");

        if !head_path.exists() {
            return self.update_ref("HEAD", new_hash);
        }

        let content = fs::read_to_string(&head_path).map_err(JogenError::Io)?;
        let content = content.trim();

        if let Some(ref_name) = content.strip_prefix("ref: ") {
            self.update_ref(ref_name, new_hash)
        } else {
            self.update_ref("HEAD", new_hash)
        }
    }

    pub fn create_track(&self, track_name: &str, hash: &str) -> Result<()> {
        let path = self.root_path.join(".jogen/refs/tracks").join(track_name);

        if path.exists() {
            return Err(JogenError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Track '{}' already exists", track_name),
            )));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(JogenError::Io)?;
        }

        fs::write(path, hash).map_err(JogenError::Io)?;

        Ok(())
    }

    pub fn list_tracks(&self) -> Result<Vec<String>> {
        let path = self.root_path.join(".jogen/refs/tracks");

        if !path.exists() {
            return Ok(vec![]);
        }

        let mut tracks = Vec::new();

        for entry in fs::read_dir(path).map_err(JogenError::Io)? {
            let entry = entry.map_err(JogenError::Io)?;

            if let Ok(name) = entry.file_name().into_string() {
                if !name.starts_with('.') {
                    tracks.push(name);
                }
            }
        }

        tracks.sort();

        Ok(tracks)
    }

    pub fn current_track(&self) -> Result<Option<String>> {
        let head_path = self.root_path.join(".jogen/HEAD");

        if !head_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(head_path).map_err(JogenError::Io)?;
        let content = content.trim();

        if let Some(ref_path) = content.strip_prefix("ref: refs/tracks/") {
            Ok(Some(ref_path.to_string()))
        } else {
            Ok(None)
        }
    }
}
