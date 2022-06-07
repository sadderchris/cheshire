use core::fmt;

use gc_arena_derive::Collect;

use crate::value::{TypeError, Value};

mod closure;
mod continuation;
mod environment;
mod function;
mod native;
mod pair;
mod port;
mod string;
mod vector;

pub use closure::ObjClosure;
pub use continuation::{ObjContinuation, Procedure};
pub use environment::{ObjEnvironment, Upvalue};
pub use function::ObjFunction;
pub use native::ObjNative;
pub use pair::ObjPair;
pub use port::{ObjReadPort, ObjWritePort};
pub use string::ObjString;
pub use vector::ObjVector;

/// Represents (mutable) boxed objects that live on the heap
#[derive(Debug, Collect)]
#[collect(no_drop)]
pub enum Object<'gc> {
    /// Closure
    Closure(ObjClosure<'gc>),

    /// Continuation
    Continuation(ObjContinuation<'gc>),

    /// Environment
    Environment(ObjEnvironment<'gc>),

    /// Function
    Function(ObjFunction<'gc>),

    /// Native function
    Native(ObjNative<'gc>),

    /// String
    String(ObjString),

    /// Pair
    Pair(ObjPair<Value<'gc>>),

    /// Vector
    Vector(ObjVector<Value<'gc>>),

    /// Input port
    ReadPort(ObjReadPort),

    /// Output port
    WritePort(ObjWritePort),
}

macro_rules! as_type {
    ($typ:ident, $val:ident) => {
        if let Self::$typ(value) = $val {
            Ok(value)
        } else {
            Err(TypeError(format!(
                "Object {} is not a {}",
                $val,
                stringify!($typ)
            )))
        }
    };
}

/// Converters
impl<'gc> Object<'gc> {
    /// Tries to turn this `Object` into a `Closure`
    pub fn as_closure(&self) -> Result<&ObjClosure<'gc>, TypeError> {
        as_type!(Closure, self)
    }

    /// Tries to turn this `Object` into a `Continuation`
    pub fn as_continuation(&self) -> Result<&ObjContinuation<'gc>, TypeError> {
        as_type!(Continuation, self)
    }

    /// Tries to turn this `Object` into an `Environment`
    pub fn as_environment(&self) -> Result<&ObjEnvironment<'gc>, TypeError> {
        as_type!(Environment, self)
    }

    /// Tries to turn this `Object` into a `Function`
    pub fn as_function(&self) -> Result<&ObjFunction<'gc>, TypeError> {
        as_type!(Function, self)
    }

    /// Tries to turn this `Object` into a `Native` function
    pub fn as_native(&self) -> Result<&ObjNative<'gc>, TypeError> {
        as_type!(Native, self)
    }

    /// Tries to turn this `Object` into a `String`
    pub fn as_string(&self) -> Result<&ObjString, TypeError> {
        as_type!(String, self)
    }

    /// Tries to turn this `Object` into a `Vector`
    pub fn as_vector(&self) -> Result<&ObjVector<Value<'gc>>, TypeError> {
        as_type!(Vector, self)
    }

    /// Tries to turn this `Object` into a `Vector`
    pub fn as_vector_mut(&mut self) -> Result<&mut ObjVector<Value<'gc>>, TypeError> {
        as_type!(Vector, self)
    }

    /// Tries to turn this `Object` into a `Pair`
    pub fn as_pair(&self) -> Result<&ObjPair<Value<'gc>>, TypeError> {
        as_type!(Pair, self)
    }

    /// Tries to turn this `Object` into a mutable `Pair`
    pub fn as_pair_mut(&mut self) -> Result<&mut ObjPair<Value<'gc>>, TypeError> {
        as_type!(Pair, self)
    }

    /// Tries to turn this `Object` into a `ReadPort`
    pub fn as_read_port(&self) -> Result<&ObjReadPort, TypeError> {
        as_type!(ReadPort, self)
    }

    /// Tries to turn this `Object` into a mutable `ReadPort`
    pub fn as_read_port_mut(&mut self) -> Result<&mut ObjReadPort, TypeError> {
        as_type!(ReadPort, self)
    }

    /// Tries to turn this `Object` into a `WritePort`
    pub fn as_write_port(&self) -> Result<&ObjWritePort, TypeError> {
        as_type!(WritePort, self)
    }

    /// Tries to turn this `Object` into a mutable `WritePort`
    pub fn as_write_port_mut(&mut self) -> Result<&mut ObjWritePort, TypeError> {
        as_type!(WritePort, self)
    }
}

/// Predicates
impl Object<'_> {
    pub fn is_closure(&self) -> bool {
        matches!(self, Object::Closure(_))
    }

    pub fn is_continuation(&self) -> bool {
        matches!(self, Object::Continuation(_))
    }

    pub fn is_environment(&self) -> bool {
        matches!(self, Object::Environment(_))
    }

    pub fn is_function(&self) -> bool {
        matches!(self, Object::Function(_))
    }

    pub fn is_native(&self) -> bool {
        matches!(self, Object::Native(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Object::String(_))
    }

    pub fn is_vector(&self) -> bool {
        matches!(self, Object::Vector(_))
    }

    pub fn is_pair(&self) -> bool {
        matches!(self, Object::Pair(_))
    }

    pub fn is_procedure(&self) -> bool {
        matches!(
            self,
            Object::Closure(_) | Object::Continuation(_) | Object::Function(_) | Object::Native(_)
        )
    }

    pub fn is_read_port(&self) -> bool {
        matches!(self, Object::ReadPort(_))
    }

    pub fn is_write_port(&self) -> bool {
        matches!(self, Object::WritePort(_))
    }
}

impl fmt::Display for Object<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closure(closure) => write!(f, "{}", closure),
            Self::Continuation(continuation) => write!(f, "{}", continuation),
            Self::Environment(environment) => write!(f, "{}", environment),
            Self::Function(function) => write!(f, "{}", function),
            Self::Native(native) => write!(f, "{}", native),
            Self::String(string) => write!(f, "{}", string),
            Self::Pair(pair) => write!(f, "{}", pair),
            Self::Vector(vector) => write!(f, "{}", vector),
            Self::ReadPort(port) => write!(f, "{}", port),
            Self::WritePort(port) => write!(f, "{}", port),
        }
    }
}
