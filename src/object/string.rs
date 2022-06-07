use core::convert::TryFrom;
use core::fmt;
use std::borrow::Cow;
use std::string::FromUtf8Error;

use gc_arena_derive::Collect;

use super::{ObjVector, Object};
use crate::value::TypeError;

/// Represents an allocated string in the VM
#[derive(Collect, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[collect(no_drop)]
pub struct ObjString {
    chars: ObjVector<u8>,
}

impl ObjString {
    pub fn new(chars: Box<[u8]>) -> Self {
        Self {
            chars: ObjVector::new(chars),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.chars.as_slice()
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.chars.as_slice_mut()
    }

    pub fn as_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }
}

impl fmt::Display for ObjString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.as_bytes();
        let string = String::from_utf8_lossy(bytes);
        write!(f, "\"{}\"", string)
    }
}

impl From<ObjString> for Object<'_> {
    fn from(value: ObjString) -> Self {
        Object::String(value)
    }
}

impl TryFrom<Object<'_>> for ObjString {
    type Error = TypeError;

    fn try_from(value: Object<'_>) -> Result<Self, Self::Error> {
        if let Object::String(string) = value {
            Ok(string)
        } else {
            Err(TypeError(format!("Object {} is not a string", value)))
        }
    }
}

impl From<String> for ObjString {
    fn from(value: String) -> Self {
        ObjString::new(value.into_bytes().into_boxed_slice())
    }
}

impl From<&'_ str> for ObjString {
    fn from(value: &str) -> Self {
        ObjString::new(value.as_bytes().into())
    }
}

impl TryFrom<ObjString> for String {
    type Error = FromUtf8Error;

    fn try_from(value: ObjString) -> Result<Self, Self::Error> {
        let items: Box<[u8]> = value.chars.into();
        String::from_utf8(items.into())
    }
}
