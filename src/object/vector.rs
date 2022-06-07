use core::convert::TryFrom;
use core::fmt;

use gc_arena_derive::Collect;

use super::Object;
use crate::value::{TypeError, Value};

/// Represents an allocated vector in the VM
#[derive(Collect, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[collect(no_drop)]
pub struct ObjVector<T> {
    items: Box<[T]>,
}

impl<T> ObjVector<T> {
    pub fn new(items: Box<[T]>) -> Self {
        Self { items }
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        &mut self.items
    }
}

impl<T: fmt::Display> fmt::Display for ObjVector<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut items = self.as_slice().iter();
        write!(f, "#(")?;
        if let Some(item) = items.next() {
            write!(f, "{}", item)?;
            for item in items {
                write!(f, " {}", item)?;
            }
        }
        write!(f, ")")
    }
}

impl<'gc> From<ObjVector<Value<'gc>>> for Object<'gc> {
    fn from(value: ObjVector<Value<'gc>>) -> Self {
        Object::Vector(value)
    }
}

impl<'gc> TryFrom<Object<'gc>> for ObjVector<Value<'gc>> {
    type Error = TypeError;

    fn try_from(value: Object<'gc>) -> Result<Self, Self::Error> {
        if let Object::Vector(vector) = value {
            Ok(vector)
        } else {
            Err(TypeError(format!("Object {} is not a string", value)))
        }
    }
}

impl<T> From<ObjVector<T>> for Box<[T]> {
    fn from(value: ObjVector<T>) -> Self {
        value.items
    }
}
