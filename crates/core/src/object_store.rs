use crate::{JogenError, Result};
use std::io::{Read, Write};
use std::{fmt, fs, path::PathBuf};
use tempfile::NamedTempFile;

const JOGEN_OBJECT_STORE_VERSION: u8 = 1;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum ObjectType {
    Blob = 1,
    Directory = 2,
    Snapshot = 3,
}

impl ObjectType {
    fn from_u8(byte: u8) -> Result<Self> {
        match byte {
            1 => Ok(Self::Blob),
            2 => Ok(Self::Directory),
            3 => Ok(Self::Snapshot),
            _ => Err(JogenError::ObjectCorrupt(format!(
                "Unknown object type byte: {}",
                byte
            ))),
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Blob => write!(f, "blob"),
            ObjectType::Directory => write!(f, "directory"),
            ObjectType::Snapshot => write!(f, "snapshot"),
        }
    }
}

pub struct ObjectHeader {
    pub version: u8,
    pub kind: ObjectType,
    pub size: u64,
}

impl ObjectHeader {
    pub const SIZE: usize = 10;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0] = self.version;
        buf[1] = self.kind as u8;
        let size_bytes = self.size.to_le_bytes();
        buf[2..10].copy_from_slice(&size_bytes);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(JogenError::ObjectCorrupt("Header too short".into()));
        }

        let version = bytes[0];
        if version != JOGEN_OBJECT_STORE_VERSION {
            return Err(JogenError::ObjectCorrupt(format!(
                "Unsupported object version: {}. Expected {}.",
                version, JOGEN_OBJECT_STORE_VERSION
            )));
        }

        let kind = ObjectType::from_u8(bytes[1])?;

        let size_bytes: [u8; 8] = bytes[2..10].try_into().unwrap();
        let size = u64::from_le_bytes(size_bytes);

        Ok(Self {
            version,
            kind,
            size,
        })
    }
}

pub struct ObjectStore {
    root_path: PathBuf,
}

impl ObjectStore {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    pub fn write_object(&self, data: &[u8], kind: ObjectType) -> Result<String> {
        let header = ObjectHeader {
            version: JOGEN_OBJECT_STORE_VERSION,
            kind,
            size: data.len() as u64,
        };

        let header_bytes = header.to_bytes();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&header_bytes);
        hasher.update(data);
        let hash = hasher.finalize().to_hex().to_string();

        let (subdir, filename) = hash.split_at(2);
        let dir_path = self.root_path.join(subdir);
        let file_path = dir_path.join(filename);

        if file_path.exists() {
            return Ok(hash);
        }

        fs::create_dir_all(&dir_path).map_err(JogenError::Io)?;
        let file = NamedTempFile::new_in(&dir_path).map_err(JogenError::Io)?;

        let file = {
            let mut encoder = zstd::stream::Encoder::new(file, 0).map_err(JogenError::Io)?;
            encoder.write_all(&header_bytes).map_err(JogenError::Io)?;
            encoder.write_all(data).map_err(JogenError::Io)?;
            encoder.finish().map_err(JogenError::Io)?
        };

        file.persist(&file_path)
            .map_err(|e| JogenError::Io(e.error))?;

        Ok(hash)
    }

    pub fn read_object(&self, hash_hex: &str) -> Result<(ObjectType, Vec<u8>)> {
        if hash_hex.len() < 2 {
            return Err(JogenError::ObjectNotFound(hash_hex.to_string()));
        }

        let (subdir, filename) = hash_hex.split_at(2);
        let file_path = self.root_path.join(subdir).join(filename);

        if !file_path.exists() {
            return Err(JogenError::ObjectNotFound(hash_hex.to_string()));
        }

        let file = fs::File::open(&file_path)?;
        let mut decoder = zstd::stream::Decoder::new(file)?;
        let mut content = Vec::new();
        decoder.read_to_end(&mut content)?;

        if content.len() < ObjectHeader::SIZE {
            return Err(JogenError::ObjectCorrupt(
                "File too small for header".into(),
            ));
        }

        let (header_bytes, data_bytes) = content.split_at(ObjectHeader::SIZE);
        let header = ObjectHeader::from_bytes(header_bytes)?;

        if data_bytes.len() as u64 != header.size {
            return Err(JogenError::ObjectCorrupt(format!(
                "Size mismatch: Header says {}, found {}",
                header.size,
                data_bytes.len()
            )));
        }

        Ok((header.kind, data_bytes.to_vec()))
    }
}
