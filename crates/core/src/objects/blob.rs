use std::borrow::Cow;

use crate::object_store::ObjectType;
use crate::objects::JogenObject;
use crate::Result;

pub struct Blob {
    pub data: Vec<u8>,
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl JogenObject for Blob {
    fn object_type(&self) -> ObjectType {
        ObjectType::Blob
    }

    fn serialize(&self) -> Result<Cow<'_, [u8]>> {
        Ok(Cow::Borrowed(&self.data))
    }
}
