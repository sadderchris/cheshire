use core::convert::TryFrom;
use core::fmt;

use gc_arena::{Collect, Gc, MutationContext};

use super::Object;
use crate::chunk::Chunk;
use crate::compiler::Upvalues;
use crate::memory::Symbol;
use crate::value::TypeError;

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub struct ObjFunction<'gc> {
    arity: usize,
    variadic: bool,
    upvalues: Gc<'gc, Upvalues>,
    chunk: Gc<'gc, Chunk<'gc>>,
    name: Option<Symbol<'gc>>,
}

impl<'gc> ObjFunction<'gc> {
    pub fn new(
        mc: MutationContext<'gc, '_>,
        arity: usize,
        variadic: bool,
        chunk: Chunk<'gc>,
        upvalues: Upvalues,
        name: Option<Symbol<'gc>>,
    ) -> Self {
        Self {
            arity,
            variadic,
            upvalues: Gc::allocate(mc, upvalues),
            chunk: Gc::allocate(mc, chunk),
            name,
        }
    }

    pub fn thunk(mc: MutationContext<'gc, '_>, chunk: Chunk<'gc>, upvalues: Upvalues) -> Self {
        Self::new(mc, 0, false, chunk, upvalues, None)
    }

    pub fn name(&self) -> Option<Symbol<'gc>> {
        self.name
    }

    pub fn chunk(&self) -> Gc<'gc, Chunk<'gc>> {
        self.chunk
    }

    pub fn arity(&self) -> usize {
        self.arity
    }

    pub fn is_variadic(&self) -> bool {
        self.variadic
    }

    pub fn upvalues(&self) -> Gc<'gc, Upvalues> {
        self.upvalues
    }
}

impl<'gc> From<ObjFunction<'gc>> for Object<'gc> {
    fn from(value: ObjFunction<'gc>) -> Self {
        Object::Function(value)
    }
}

impl<'gc> TryFrom<Object<'gc>> for ObjFunction<'gc> {
    type Error = TypeError;

    fn try_from(value: Object<'gc>) -> Result<Self, Self::Error> {
        if let Object::Function(function) = value {
            Ok(function)
        } else {
            Err(TypeError(format!("")))
        }
    }
}

impl fmt::Display for ObjFunction<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name.as_ref() {
            write!(f, "#<procedure {}>", name)
        } else {
            write!(f, "#<anonymous procedure {:p}>", self)
        }
    }
}
