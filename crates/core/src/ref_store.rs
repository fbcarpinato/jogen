use crate::{JogenError, Result};
use std::{fs, path::PathBuf};

pub struct RefStore {
    root_path: PathBuf,
}

pub struct IntegrationStatus {
    pub base_hash: String,
    pub target_hash: String,
    pub target_name: String,
    pub conflict_paths: Vec<String>,
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

    pub fn resolve_track(&self, track_name: &str) -> Result<Option<String>> {
        self.read_ref(&format!("refs/tracks/{}", track_name))
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

    pub fn set_head_to_track(&self, track_name: &str) -> Result<()> {
        let head_path = self.root_path.join(".jogen/HEAD");
        let content = format!("ref: refs/tracks/{}\n", track_name);
        fs::write(head_path, content).map_err(JogenError::Io)?;
        Ok(())
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

    pub fn switch_track(&self, track_name: &str) -> Result<Option<String>> {
        let path = self.root_path.join(".jogen/refs/tracks").join(track_name);

        if !path.exists() {
            return Err(JogenError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Track '{}' does not exist", track_name),
            )));
        }

        let hash = fs::read_to_string(path).map_err(JogenError::Io)?;
        self.set_head_to_track(track_name)?;

        Ok(Some(hash.trim().to_string()))
    }

    pub fn begin_integration(
        &self,
        base_hash: &str,
        target_hash: &str,
        target_name: &str,
        conflict_paths: &[String],
    ) -> Result<()> {
        let path = self.root_path.join(".jogen/INTEGRATING");
        let mut content = format!(
            "{}\n{}\n{}\nconflicts {}",
            base_hash,
            target_hash,
            target_name,
            conflict_paths.len()
        );
        for conflict_path in conflict_paths {
            content.push('\n');
            content.push_str(conflict_path);
        }
        fs::write(path, content).map_err(JogenError::Io)?;
        Ok(())
    }

    pub fn get_integration_status(&self) -> Result<Option<IntegrationStatus>> {
        let path = self.root_path.join(".jogen/INTEGRATING");
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path).map_err(JogenError::Io)?;
        let mut lines = content.lines();
        let base = lines.next().unwrap_or("").to_string();
        let target_hash = lines.next().unwrap_or("").to_string();
        let target_name = lines.next().unwrap_or("").to_string();

        if target_hash.is_empty() {
            return Ok(None);
        }

        let mut conflict_paths = Vec::new();
        if let Some(conflict_header) = lines.next() {
            if let Some(count_str) = conflict_header.strip_prefix("conflicts ") {
                let count = count_str.parse::<usize>().unwrap_or(0);
                for _ in 0..count {
                    if let Some(path) = lines.next() {
                        conflict_paths.push(path.to_string());
                    }
                }
            }
        }

        Ok(Some(IntegrationStatus {
            base_hash: base,
            target_hash,
            target_name,
            conflict_paths,
        }))
    }

    pub fn clear_integration(&self) -> Result<()> {
        let path = self.root_path.join(".jogen/INTEGRATING");
        if path.exists() {
            fs::remove_file(path).map_err(JogenError::Io)?;
        }
        Ok(())
    }
}
