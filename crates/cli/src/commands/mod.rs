use anyhow::Result;
use std::path::PathBuf;

use jogen_core::{object_store::ObjectStore, ref_store::RefStore};

pub mod actions;
pub mod tools;

struct JogenRepo {
    root_path: PathBuf,
    object_store: ObjectStore,
    ref_store: RefStore,
}

impl JogenRepo {
    fn from_cwd() -> Result<Self> {
        let root_path = jogen_core::find_root_from_cwd()?;
        let objects_dir = root_path.join(".jogen").join("objects");
        let object_store = ObjectStore::new(objects_dir);
        let ref_store = RefStore::new(root_path.clone());
        Ok(Self {
            root_path,
            object_store,
            ref_store,
        })
    }
}
