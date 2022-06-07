use core::convert::TryFrom;
use core::fmt;

use gc_arena::Gc;
use gc_arena_derive::Collect;

use super::{ObjEnvironment, ObjFunction, Object};
use crate::value::TypeError;

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub struct ObjClosure<'gc> {
    function: ObjFunction<'gc>,
    environment: Gc<'gc, ObjEnvironment<'gc>>,
    upvalue_count: usize,
}

impl<'gc> ObjClosure<'gc> {
    pub fn new(function: ObjFunction<'gc>, environment: Gc<'gc, ObjEnvironment<'gc>>) -> Self {
        let upvalue_count = function.upvalues().len();
        Self {
            function,
            environment,
            upvalue_count,
        }
    }

    pub fn function(&self) -> &ObjFunction<'gc> {
        &self.function
    }

    pub fn is_variadic(&self) -> bool {
        self.function.is_variadic()
    }

    pub fn arity(&self) -> usize {
        self.function.arity()
    }

    pub fn upvalue_count(&self) -> usize {
        self.upvalue_count
    }

    pub fn environment(&self) -> Gc<'gc, ObjEnvironment<'gc>> {
        self.environment
    }
}

impl<'gc> From<ObjClosure<'gc>> for Object<'gc> {
    fn from(value: ObjClosure<'gc>) -> Self {
        Object::Closure(value)
    }
}

impl<'gc> TryFrom<Object<'gc>> for ObjClosure<'gc> {
    type Error = TypeError;

    fn try_from(value: Object<'gc>) -> Result<Self, Self::Error> {
        if let Object::Closure(function) = value {
            Ok(function)
        } else {
            Err(TypeError(format!("")))
        }
    }
}

impl fmt::Display for ObjClosure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.function)
    }
}
