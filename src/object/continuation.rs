use core::convert::TryFrom;
use core::fmt;

use gc_arena::GcCell;
use gc_arena_derive::Collect;

use super::{ObjClosure, ObjFunction, ObjNative, Object};
use crate::value::TypeError;
use crate::vm::Stack;

/// Represents an ongoing execution of a procedure
#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub enum Procedure<'gc> {
    /// Closure
    Closure {
        /// Closure
        closure: ObjClosure<'gc>,

        /// Offset to currently executing OpCode within function
        ip: usize,
    },
    /// Function
    Function {
        /// Function
        function: ObjFunction<'gc>,

        /// Offset to currently executing OpCode within function
        ip: usize,
    },
    /// Native function
    Native(ObjNative<'gc>),
}

/// Representation of a function invokation, a currently executing function
#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub struct ObjContinuation<'gc> {
    /// Call frames/continuations
    frames: Option<GcCell<'gc, ObjContinuation<'gc>>>,

    /// Currently executing procedure
    procedure: Procedure<'gc>,

    /// Local stack
    stack: Stack<'gc>,

    /// Top of the stack
    stack_top: usize,

    /// Current input port
    current_input_port: GcCell<'gc, Object<'gc>>,

    /// Current output port
    current_output_port: GcCell<'gc, Object<'gc>>,
}

impl<'gc> ObjContinuation<'gc> {
    /// Creates a new continuation
    pub fn new(
        frames: Option<GcCell<'gc, ObjContinuation<'gc>>>,
        procedure: Procedure<'gc>,
        stack: Stack<'gc>,
        current_input_port: GcCell<'gc, Object<'gc>>,
        current_output_port: GcCell<'gc, Object<'gc>>,
    ) -> Self {
        Self {
            frames,
            procedure,
            stack,
            stack_top: stack.read().len(),
            current_input_port,
            current_output_port,
        }
    }

    /// Gets this continuation's frames
    pub fn frames(&self) -> Option<GcCell<'gc, ObjContinuation<'gc>>> {
        self.frames
    }

    /// Gets the procedure associated with this continuation
    pub fn procedure(&self) -> &Procedure<'gc> {
        &self.procedure
    }

    /// Gets the stack associated with this continuation
    pub fn stack(&self) -> Stack<'gc> {
        self.stack
    }

    /// Gets the length of the stack at the time the continuation was created
    pub fn stack_top(&self) -> usize {
        self.stack_top
    }

    /// Gets the current input port
    pub fn current_input_port(&self) -> GcCell<'gc, Object<'gc>> {
        self.current_input_port
    }

    /// Gets the current output port
    pub fn current_output_port(&self) -> GcCell<'gc, Object<'gc>> {
        self.current_output_port
    }
}

impl<'gc> From<ObjContinuation<'gc>> for Object<'gc> {
    fn from(value: ObjContinuation<'gc>) -> Self {
        Object::Continuation(value)
    }
}

impl<'gc> TryFrom<Object<'gc>> for ObjContinuation<'gc> {
    type Error = TypeError;

    fn try_from(value: Object<'gc>) -> Result<Self, Self::Error> {
        if let Object::Continuation(continuation) = value {
            Ok(continuation)
        } else {
            Err(TypeError(format!("")))
        }
    }
}

impl fmt::Display for ObjContinuation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#<continuation>")
    }
}
