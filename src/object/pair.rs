use core::convert::TryFrom;
use core::fmt;

use gc_arena_derive::Collect;

use super::Object;
use crate::value::{Datum, TypeError, Value};

#[derive(Debug, Clone, Collect, PartialEq, Eq)]
#[collect(no_drop)]
pub struct ObjPair<T>(T, T);

impl<T> ObjPair<T> {
    pub fn new(car: T, cdr: T) -> Self {
        Self(car, cdr)
    }

    pub fn as_car(&self) -> &T {
        &self.0
    }

    pub fn as_cdr(&self) -> &T {
        &self.1
    }

    pub fn set_car(&mut self, car: T) {
        self.0 = car
    }

    pub fn set_cdr(&mut self, cdr: T) {
        self.1 = cdr
    }
}

impl<T: Copy> ObjPair<T> {
    pub fn car(&self) -> T {
        self.0
    }

    pub fn cdr(&self) -> T {
        self.1
    }
}

impl<'gc> From<ObjPair<Value<'gc>>> for Object<'gc> {
    fn from(value: ObjPair<Value<'gc>>) -> Self {
        Object::Pair(value)
    }
}

impl<'gc> TryFrom<Object<'gc>> for ObjPair<Value<'gc>> {
    type Error = TypeError;

    fn try_from(value: Object<'gc>) -> Result<Self, Self::Error> {
        if let Object::Pair(pair) = value {
            Ok(pair)
        } else {
            Err(TypeError(format!("'{}' is not a pair", value)))
        }
    }
}

impl fmt::Display for ObjPair<Value<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cdr = self.cdr();
        write!(f, "({}", self.car())?;
        while !cdr.is_null() {
            match cdr {
                Value::Box(object) => match &*object.read() {
                    Object::Pair(pair) => {
                        write!(f, " {}", pair.car())?;
                        cdr = pair.cdr();
                        continue;
                    }
                    _ => {
                        write!(f, " . {}", cdr)?;
                        break;
                    }
                },
                _ => {
                    write!(f, " . {}", cdr)?;
                    break;
                }
            }
        }
        write!(f, ")")
    }
}

impl fmt::Display for ObjPair<Datum<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cdr = self.cdr();
        write!(f, "({}", self.car())?;
        while !cdr.is_null() {
            match cdr {
                Datum::Pair(pair) => {
                    write!(f, " {}", pair.car())?;
                    cdr = pair.cdr();
                    continue;
                }
                _ => {
                    write!(f, " . {}", cdr)?;
                    break;
                }
            }
        }
        write!(f, ")")
    }
}
