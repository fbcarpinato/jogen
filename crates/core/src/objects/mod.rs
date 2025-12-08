use std::borrow::Cow;

use super::object_store::ObjectType;
use crate::Result;

pub mod blob;
pub mod directory;
pub mod snapshot;

pub trait JogenObject {
    fn object_type(&self) -> ObjectType;
    fn serialize(&self) -> Result<Cow<'_, [u8]>>;
}
