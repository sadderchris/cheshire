use core::convert::TryFrom;
use core::fmt;
use core::ops::Deref;

use gc_arena::{Gc, GcCell, MutationContext};
use gc_arena_derive::Collect;
use thiserror::Error;

use crate::memory::Symbol;
use crate::object::{ObjPair, ObjString, ObjVector, Object};
use crate::vm::InterpretError;

#[derive(Collect, Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[collect(require_static)]
pub struct Char(pub char);

impl Deref for Char {
    type Target = char;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.0
    }
}

impl fmt::Display for Char {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Copy, Clone, Collect)]
#[collect(no_drop)]
pub enum Datum<'gc> {
    Bool(bool),
    Char(Char),
    Number(f64),
    Pair(Gc<'gc, ObjPair<Datum<'gc>>>),
    String(Gc<'gc, ObjString>),
    Symbol(Symbol<'gc>),
    Vector(Gc<'gc, ObjVector<Datum<'gc>>>),
    Null,
    Eof,
}

impl Datum<'_> {
    pub fn character(character: char) -> Self {
        Self::Char(Char(character))
    }
}

impl<'gc> Datum<'gc> {
    /// Similar to directy converting to a [Value], maps allocated types (string, pair,
    /// etc.) to boxed heap types instead of const types
    pub fn into_boxed_value(self, mc: MutationContext<'gc, '_>) -> Value<'gc> {
        match self {
            Datum::Bool(b) => Value::Bool(b),
            Datum::Char(c) => Value::Char(c),
            Datum::Number(n) => Value::Number(n),
            Datum::Pair(p) => {
                let car = p.car().into_boxed_value(mc);
                let cdr = p.cdr().into_boxed_value(mc);
                Value::Box(GcCell::allocate(mc, Object::Pair(ObjPair::new(car, cdr))))
            }
            Datum::String(s) => Value::Box(GcCell::allocate(mc, Object::String(s.deref().clone()))),
            Datum::Symbol(s) => Value::Symbol(s),
            Datum::Vector(v) => {
                let values: Vec<_> = v
                    .as_slice()
                    .iter()
                    .map(|datum| datum.into_boxed_value(mc))
                    .collect();
                Value::Box(GcCell::allocate(
                    mc,
                    Object::Vector(ObjVector::new(values.into_boxed_slice())),
                ))
            }
            Datum::Null => Value::Null,
            Datum::Eof => Value::Eof,
        }
    }
}

/// Predicates
impl Datum<'_> {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Self::Bool(_b @ false))
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_falsey()
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    pub fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol(_))
    }

    pub fn is_pair(&self) -> bool {
        matches!(self, Self::Pair(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }
}

/// Conversions
impl<'gc> Datum<'gc> {
    pub fn as_pair(&self) -> Result<Gc<'gc, ObjPair<Datum<'gc>>>, TypeError> {
        if let Self::Pair(pair) = self {
            Ok(*pair)
        } else {
            Err(TypeError(format!("'{}' is not a pair", self)))
        }
    }

    pub fn as_symbol(&self) -> Result<Symbol<'gc>, TypeError> {
        if let Self::Symbol(symbol) = self {
            Ok(*symbol)
        } else {
            Err(TypeError(format!("'{}' is not a symbol", self)))
        }
    }
}

impl From<bool> for Datum<'_> {
    fn from(value: bool) -> Self {
        Datum::Bool(value)
    }
}

impl From<char> for Datum<'_> {
    fn from(value: char) -> Self {
        Datum::Char(Char(value))
    }
}

impl From<f64> for Datum<'_> {
    fn from(value: f64) -> Self {
        Datum::Number(value)
    }
}

impl<'gc> From<Gc<'gc, ObjPair<Datum<'gc>>>> for Datum<'gc> {
    fn from(value: Gc<'gc, ObjPair<Datum<'gc>>>) -> Self {
        Datum::Pair(value)
    }
}

impl<'gc> From<Gc<'gc, ObjString>> for Datum<'gc> {
    fn from(value: Gc<'gc, ObjString>) -> Self {
        Datum::String(value)
    }
}

impl<'gc> From<Symbol<'gc>> for Datum<'gc> {
    fn from(value: Symbol<'gc>) -> Self {
        Datum::Symbol(value)
    }
}

impl<'gc> From<Gc<'gc, ObjVector<Datum<'gc>>>> for Datum<'gc> {
    fn from(value: Gc<'gc, ObjVector<Datum<'gc>>>) -> Self {
        Datum::Vector(value)
    }
}

/// Represents a value within the virtual machine
#[derive(Debug, Copy, Clone, Collect)]
#[collect(no_drop)]
pub enum Value<'gc> {
    Bool(bool),
    Char(Char),
    Number(f64),
    Pair(Gc<'gc, ObjPair<Datum<'gc>>>),
    String(Gc<'gc, ObjString>),
    Box(GcCell<'gc, Object<'gc>>),
    Symbol(Symbol<'gc>),
    Vector(Gc<'gc, ObjVector<Datum<'gc>>>),
    Eof,
    Null,
    Void,
}

#[derive(Debug, Error)]
pub struct TypeError(pub String);

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<TypeError> for InterpretError {
    fn from(value: TypeError) -> Self {
        InterpretError::RuntimeError(value.0)
    }
}

impl Value<'_> {
    pub fn null() -> Self {
        Self::Null
    }

    pub fn character(character: char) -> Self {
        Self::Char(Char(character))
    }
}

impl<'gc> Value<'gc> {
    pub fn boxed(mc: MutationContext<'gc, '_>, object: Object<'gc>) -> Self {
        Self::Box(GcCell::allocate(mc, object))
    }
}

/// Conversions
impl Value<'_> {
    pub fn as_bool(&self) -> Result<bool, TypeError> {
        if let Self::Bool(boolean) = self {
            Ok(*boolean)
        } else {
            Err(TypeError(format!("'{}' is not a boolean", self)))
        }
    }

    pub fn as_char(&self) -> Result<char, TypeError> {
        if let Self::Char(character) = self {
            Ok(character.0)
        } else {
            Err(TypeError(format!("'{}' is not a character", self)))
        }
    }

    pub fn as_number(&self) -> Result<f64, TypeError> {
        if let Self::Number(number) = self {
            Ok(*number)
        } else {
            Err(TypeError(format!("'{}' is not a number", self)))
        }
    }
}

/// Conversions
impl<'gc> Value<'gc> {
    pub fn as_object(&self) -> Result<GcCell<'gc, Object<'gc>>, TypeError> {
        if let Self::Box(object) = self {
            Ok(*object)
        } else {
            Err(TypeError(format!("'{}' is not an object", self)))
        }
    }

    pub fn as_symbol(&self) -> Result<Symbol<'gc>, TypeError> {
        if let Self::Symbol(symbol) = self {
            Ok(*symbol)
        } else {
            Err(TypeError(format!("'{}' is not a symbol", self)))
        }
    }
}

/// Predicates
impl Value<'_> {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Self::Bool(_b @ false))
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_falsey()
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    pub fn is_eof(&self) -> bool {
        matches!(self, Self::Eof)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Box(_))
    }

    pub fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol(_))
    }
}

impl TryFrom<Value<'_>> for f64 {
    type Error = TypeError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        if let Value::Number(number) = value {
            Ok(number)
        } else {
            Err(TypeError(format!("'{}' is not a number", value)))
        }
    }
}

impl TryFrom<Value<'_>> for bool {
    type Error = TypeError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        if let Value::Bool(boolean) = value {
            Ok(boolean)
        } else {
            Err(TypeError(format!("'{}' is not a boolean", value)))
        }
    }
}

impl<'gc> From<Datum<'gc>> for Value<'gc> {
    fn from(value: Datum<'gc>) -> Self {
        match value {
            Datum::Bool(b) => Value::Bool(b),
            Datum::Char(c) => Value::Char(c),
            Datum::Number(n) => Value::Number(n),
            Datum::Pair(p) => Value::Pair(p),
            Datum::String(s) => Value::String(s),
            Datum::Symbol(s) => Value::Symbol(s),
            Datum::Vector(v) => Value::Vector(v),
            Datum::Null => Value::Null,
            Datum::Eof => Value::Eof,
        }
    }
}

impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Bool(_b @ true) => {
                write!(f, "#t")
            }
            Self::Bool(_b @ false) => {
                write!(f, "#f")
            }
            Self::Pair(pair) => write!(f, "{}", *pair),
            Self::String(string) => write!(f, "{}", *string),
            Self::Box(object) => write!(f, "{}", object.read()),
            Self::Char(Char(_c @ ' ')) => {
                write!(f, "#\\space")
            }
            Self::Char(Char(_c @ '\n')) => {
                write!(f, "#\\newline")
            }
            Self::Char(Char(character)) => {
                write!(f, "#\\{}", character)
            }
            Self::Number(number) => write!(f, "{}", number),
            Self::Symbol(symbol) => write!(f, "{}", symbol),
            Self::Vector(vector) => write!(f, "{}", *vector),
            Self::Eof => write!(f, "#<eof>"),
            Self::Null => write!(f, "()"),
            Self::Void => write!(f, "#<void>"),
        }
    }
}

impl fmt::Display for Datum<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Bool(_b @ true) => {
                write!(f, "#t")
            }
            Self::Bool(_b @ false) => {
                write!(f, "#f")
            }
            Self::Pair(pair) => write!(f, "{}", *pair),
            Self::String(string) => write!(f, "{}", *string),
            Self::Char(Char(_c @ ' ')) => {
                write!(f, "#\\space")
            }
            Self::Char(Char(_c @ '\n')) => {
                write!(f, "#\\newline")
            }
            Self::Char(Char(character)) => {
                write!(f, "#\\{}", character)
            }
            Self::Number(number) => write!(f, "{}", number),
            Self::Symbol(symbol) => write!(f, "{}", symbol),
            Self::Vector(vector) => write!(f, "{}", *vector),
            Self::Null => write!(f, "()"),
            Self::Eof => write!(f, "#<eof>"),
        }
    }
}
