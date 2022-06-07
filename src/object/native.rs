use core::fmt;

use gc_arena::{Collect, MutationContext};

use crate::memory::Symbol;
use crate::value::Value;
use crate::vm::{InterpretError, Stack, VirtualMachine};

type Native = for<'gc> fn(
    &VirtualMachine<'gc>,
    Stack<'gc>,
    MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>, InterpretError>;

/// Representation of a native function
#[derive(Copy, Clone, Collect)]
#[collect(require_static)]
pub struct NativeFn(Native);

impl NativeFn {
    pub fn call<'gc>(
        &self,
        vm: &VirtualMachine<'gc>,
        args: Stack<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Result<Option<Value<'gc>>, InterpretError> {
        (self.0)(vm, args, mc)
    }
}

impl fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NativeFn")
            .field(&(&self.0 as *const Native))
            .finish()
        // write!(f, "NativeFn(0x{:x})", self.0 as usize)
    }
}

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub struct ObjNative<'gc> {
    arity: usize,
    variadic: bool,
    function: NativeFn,
    name: Option<Symbol<'gc>>,
}

impl ObjNative<'_> {
    pub fn arity(&self) -> usize {
        self.arity
    }

    pub fn is_variadic(&self) -> bool {
        self.variadic
    }

    pub fn call<'gc>(
        &self,
        vm: &VirtualMachine<'gc>,
        args: Stack<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Result<Option<Value<'gc>>, InterpretError> {
        self.function.call(vm, args, mc)
    }
}

impl<'gc> ObjNative<'gc> {
    pub fn new(arity: usize, variadic: bool, function: Native, name: Option<Symbol<'gc>>) -> Self {
        Self {
            arity,
            variadic,
            function: NativeFn(function),
            name,
        }
    }
}

impl fmt::Display for ObjNative<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name {
            write!(f, "#<native procedure {}>", name)
        } else {
            write!(f, "#<anonymous native procedure>")
        }
    }
}
