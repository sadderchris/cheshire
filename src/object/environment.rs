use core::convert::TryFrom;
use core::fmt;

use gc_arena::MutationContext;
use gc_arena_derive::Collect;

use super::Object;
use crate::value::{TypeError, Value};
use crate::vm::Stack;

#[derive(Debug, Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct Upvalue<'gc> {
    stack: Stack<'gc>,
    offset: usize,
}

impl<'gc> Upvalue<'gc> {
    pub fn new(stack: Stack<'gc>, offset: usize) -> Self {
        Upvalue { stack, offset }
    }

    pub fn location(&self) -> Value<'gc> {
        self.stack.read()[self.offset]
    }

    pub fn set_location(&self, value: Value<'gc>, mc: MutationContext<'gc, '_>) {
        self.stack.write(mc)[self.offset] = value;
    }
}

/// Represents the captured environment that a closure executes in
#[derive(Debug, Default, Clone, Collect)]
#[collect(no_drop)]
pub struct ObjEnvironment<'gc> {
    upvalues: Vec<Upvalue<'gc>>,
}

impl<'gc> ObjEnvironment<'gc> {
    /// Creates a new environment
    pub fn new(upvalues: Vec<Upvalue<'gc>>) -> Self {
        Self { upvalues }
    }

    /// Captures an `Upvalue` into this environment
    pub fn capture_upvalue(&mut self, stack: Stack<'gc>, offset: usize) {
        self.upvalues.push(Upvalue { stack, offset });
    }

    pub(crate) fn upvalues(&self) -> &[Upvalue<'gc>] {
        &self.upvalues
    }
}

impl<'gc> From<ObjEnvironment<'gc>> for Object<'gc> {
    fn from(value: ObjEnvironment<'gc>) -> Self {
        Object::Environment(value)
    }
}

impl<'gc> TryFrom<Object<'gc>> for ObjEnvironment<'gc> {
    type Error = TypeError;

    fn try_from(value: Object<'gc>) -> Result<Self, Self::Error> {
        if let Object::Environment(environment) = value {
            Ok(environment)
        } else {
            Err(TypeError(format!("")))
        }
    }
}

impl fmt::Display for ObjEnvironment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#<environment>")
    }
}
