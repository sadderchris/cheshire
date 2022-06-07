use core::cell::Cell;
use core::convert::TryFrom;
use core::str::Utf8Error;
use std::collections::HashMap;
use std::io;

use gc_arena::{Gc, GcCell, MutationContext};
use gc_arena_derive::Collect;
use pest::error::Error;
use thiserror::Error;

use crate::builtins;
use crate::chunk::{Chunk, OpCode};
use crate::compiler::bootstrap;
use crate::memory::{Symbol, SymbolTable, Token};
use crate::object::{
    self, ObjClosure, ObjContinuation, ObjEnvironment, ObjFunction, ObjNative, ObjPair,
    ObjReadPort, ObjString, ObjWritePort, Object, Upvalue,
};
use crate::scanner::Rule;
use crate::value::{TypeError, Value};

const STACK_MAX: usize = u8::MAX as usize + 1;

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub(crate) enum Procedure<'gc> {
    Closure(ObjClosure<'gc>),
    Function(ObjFunction<'gc>),
    Native(ObjNative<'gc>),
}

impl<'gc> TryFrom<Value<'gc>> for Procedure<'gc> {
    type Error = TypeError;

    fn try_from(value: Value<'gc>) -> std::result::Result<Self, Self::Error> {
        use Value::*;
        match value {
            Box(b) => match &*b.read() {
                Object::Native(n) => Ok(Procedure::Native(n.clone())),
                Object::Function(f) => Ok(Procedure::Function(f.clone())),
                Object::Closure(c) => Ok(Procedure::Closure(c.clone())),
                _ => Err(TypeError(format!("'{}' is not a procedure", value))),
            },
            _ => Err(TypeError(format!("'{}' is not a procedure", value))),
        }
    }
}

pub(crate) type Stack<'gc> = GcCell<'gc, Vec<Value<'gc>>>;

/// Represents the VM that our language executes on
#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct VirtualMachine<'gc> {
    /// Parent continuation (frame)
    parent_continuation: GcCell<'gc, Option<GcCell<'gc, ObjContinuation<'gc>>>>,

    /// Currently executing procedure
    procedure: GcCell<'gc, Procedure<'gc>>,

    /// Instruction pointer
    ip: Cell<usize>,

    /// Stack
    stack: GcCell<'gc, Stack<'gc>>,

    /// Symbol pool
    symbol_pool: GcCell<'gc, SymbolTable<'gc>>,

    /// Global variable table
    globals: GcCell<'gc, HashMap<Symbol<'gc>, Value<'gc>>>,

    /// Current input port
    current_input_port: GcCell<'gc, GcCell<'gc, Object<'gc>>>,

    /// Current output port
    current_output_port: GcCell<'gc, GcCell<'gc, Object<'gc>>>,
}

/// Represents an error from the interpreter
#[derive(Error, Debug)]
pub enum InterpretError {
    /// Compiler error
    #[error("compile error: {0}")]
    CompileError(#[from] Error<Rule>),

    /// Runtime error
    #[error("runtime error: {0}")]
    RuntimeError(String),

    #[error("io error: {0}")]
    IoError(#[from] io::Error),

    #[error("utf8 error: {0}")]
    Utf8Error(#[from] Utf8Error),

    #[error("{0}")]
    CompilerError(#[from] bootstrap::CompileError),
}

/// Represents the result of executing the interpreter on an expression
pub type Result<T> = std::result::Result<T, InterpretError>;

macro_rules! define_native {
    ($vm:ident, $mc:ident, $name:literal, $native:expr, $arity:literal, $variadic:literal) => {
        let name = $vm.intern_symbol(Token::new($mc, $name.into()), $mc);
        $vm.define_global(
            name,
            Value::boxed(
                $mc,
                Object::Native(ObjNative::new($arity, $variadic, $native, Some(name))),
            ),
            $mc,
        );
    };
}

impl<'gc> VirtualMachine<'gc> {
    /// Construct a new VM
    pub fn new(mc: MutationContext<'gc, '_>) -> Self {
        Self {
            parent_continuation: GcCell::allocate(mc, None),
            procedure: GcCell::allocate(
                mc,
                Procedure::Native(ObjNative::new(0, false, builtins::exit, None)),
            ),
            ip: Cell::new(0),
            stack: GcCell::allocate(mc, GcCell::allocate(mc, Vec::with_capacity(STACK_MAX))),
            symbol_pool: GcCell::allocate(mc, SymbolTable::default()),
            globals: GcCell::allocate(mc, HashMap::default()),
            current_input_port: GcCell::allocate(
                mc,
                GcCell::allocate(mc, Object::ReadPort(ObjReadPort::new(io::stdin()))),
            ),
            current_output_port: GcCell::allocate(
                mc,
                GcCell::allocate(mc, Object::WritePort(ObjWritePort::new(io::stdout()))),
            ),
        }
    }

    pub fn default(mc: MutationContext<'gc, '_>) -> Self {
        let vm = Self::new(mc);

        define_native!(vm, mc, "pair?", builtins::is_pair, 1, false);
        define_native!(vm, mc, "cons", builtins::cons, 2, false);
        define_native!(vm, mc, "car", builtins::car, 1, false);
        define_native!(vm, mc, "cdr", builtins::cdr, 1, false);
        define_native!(vm, mc, "set-car!", builtins::set_car, 2, false);
        define_native!(vm, mc, "set-cdr!", builtins::set_cdr, 2, false);
        define_native!(vm, mc, "number?", builtins::is_number, 1, false);
        define_native!(vm, mc, "symbol?", builtins::is_symbol, 1, false);
        define_native!(vm, mc, "char?", builtins::is_char, 1, false);
        define_native!(vm, mc, "string?", builtins::is_string, 1, false);
        define_native!(vm, mc, "vector?", builtins::is_vector, 1, false);
        define_native!(vm, mc, "procedure?", builtins::is_procedure, 1, false);
        define_native!(vm, mc, "+", builtins::plus, 1, true);
        define_native!(vm, mc, "-", builtins::minus, 1, true);
        define_native!(vm, mc, "*", builtins::multiply, 1, true);
        define_native!(vm, mc, "/", builtins::divide, 1, true);
        define_native!(vm, mc, "=", builtins::equal_number, 3, true);
        define_native!(vm, mc, "<", builtins::lt_number, 3, true);
        define_native!(vm, mc, ">", builtins::gt_number, 3, true);
        define_native!(vm, mc, "<=", builtins::lte_number, 3, true);
        define_native!(vm, mc, ">=", builtins::gte_number, 3, true);
        define_native!(vm, mc, "eqv?", builtins::is_eqv, 2, false);
        define_native!(vm, mc, "eq?", builtins::is_eq, 2, false);
        define_native!(vm, mc, "char=?", builtins::is_char_eq, 2, false);
        define_native!(vm, mc, "char<?", builtins::is_char_lt, 2, false);
        define_native!(vm, mc, "char>?", builtins::is_char_gt, 2, false);
        define_native!(vm, mc, "char<=?", builtins::is_char_lte, 2, false);
        define_native!(vm, mc, "char>=?", builtins::is_char_gte, 2, false);
        define_native!(
            vm,
            mc,
            "char-alphabetic?",
            builtins::is_char_alphabetic,
            1,
            false
        );
        define_native!(vm, mc, "char-numeric?", builtins::is_char_numeric, 1, false);
        define_native!(
            vm,
            mc,
            "char-whitespace?",
            builtins::is_char_whitespace,
            1,
            false
        );
        define_native!(
            vm,
            mc,
            "char-upper-case?",
            builtins::is_char_upper_case,
            1,
            false
        );
        define_native!(
            vm,
            mc,
            "char-lower-case?",
            builtins::is_char_lower_case,
            1,
            false
        );
        define_native!(vm, mc, "char-upcase", builtins::char_upcase, 1, false);
        define_native!(vm, mc, "char-downcase", builtins::char_downcase, 1, false);
        define_native!(
            vm,
            mc,
            "symbol->string",
            builtins::symbol_to_string,
            1,
            false
        );
        define_native!(
            vm,
            mc,
            "string->symbol",
            builtins::string_to_symbol,
            1,
            false
        );
        define_native!(vm, mc, "make-string", builtins::make_string, 2, true);
        define_native!(vm, mc, "string-length", builtins::string_length, 1, false);
        define_native!(vm, mc, "make-vector", builtins::make_vector, 2, true);
        define_native!(vm, mc, "vector-length", builtins::vector_length, 1, false);
        define_native!(vm, mc, "vector-ref", builtins::vector_ref, 2, false);
        define_native!(vm, mc, "vector-set!", builtins::vector_set, 3, false);
        define_native!(vm, mc, "apply", builtins::apply, 2, true);
        define_native!(
            vm,
            mc,
            "call-with-current-continuation",
            builtins::call_with_current_continuation,
            1,
            false
        );
        define_native!(vm, mc, "values", builtins::values, 1, true);
        define_native!(
            vm,
            mc,
            "call-with-values",
            builtins::call_with_values,
            2,
            false
        );
        define_native!(vm, mc, "input-port?", builtins::is_input_port, 1, false);
        define_native!(vm, mc, "output-port?", builtins::is_output_port, 1, false);
        define_native!(
            vm,
            mc,
            "current-input-port",
            builtins::current_input_port,
            0,
            false
        );
        define_native!(
            vm,
            mc,
            "current-output-port",
            builtins::current_output_port,
            0,
            false
        );
        define_native!(vm, mc, "read-char", builtins::read_char, 0, true);
        define_native!(vm, mc, "peek-char", builtins::peek_char, 0, true);
        define_native!(vm, mc, "eof-object?", builtins::is_eof_object, 1, false);
        define_native!(vm, mc, "char-ready?", builtins::is_char_ready, 0, true);
        define_native!(vm, mc, "write-char", builtins::write_char, 1, true);
        define_native!(vm, mc, "read", builtins::read, 0, true);
        define_native!(vm, mc, "compile", builtins::compile, 1, false);
        define_native!(vm, mc, "load", builtins::load, 1, false);
        define_native!(vm, mc, "exit", builtins::exit, 0, false);
        define_native!(vm, mc, "disassemble", builtins::disassemble, 1, false);
        vm
    }

    pub fn repl(mc: MutationContext<'gc, '_>) -> Self {
        let vm = Self::default(mc);

        let repl = Value::boxed(
            mc,
            Object::Native(ObjNative::new(0, false, builtins::read_thunk, None)),
        );

        let stack = *vm.stack.read();
        stack.write(mc).push(repl);
        vm.call_value(repl, stack, 0, mc)
            .expect("Failed to call the repl");
        vm
    }

    pub fn reset_repl(&self, mc: MutationContext<'gc, '_>) {
        *self.parent_continuation.write(mc) = None;
        *self.procedure.write(mc) = Procedure::Native(ObjNative::new(0, false, builtins::exit, None));

        let repl = Value::boxed(
            mc,
            Object::Native(ObjNative::new(0, false, builtins::read_thunk, None)),
        );

        let stack = *self.stack.read();
        stack.write(mc).push(repl);
        self.call_value(repl, stack, 0, mc)
            .expect("Failed to call the repl");
    }

    pub fn load_file(path: String, mc: MutationContext<'gc, '_>) -> Self {
        let vm = Self::default(mc);

        let load_symbol = vm.symbol_pool.write(mc).intern(Token::new(mc, ObjString::from("load")));
        let load = *vm.globals.read().get(&load_symbol).unwrap();

        let stack = *vm.stack.read();
        stack.write(mc).push(load);
        stack
            .write(mc)
            .push(Value::String(Gc::allocate(mc, ObjString::from(path))));

        vm.call_value(load, stack, 1, mc)
            .expect("Failed to load program");
        vm
    }

    fn save_current_continuation(&self) -> ObjContinuation<'gc> {
        let procedure = match &*self.procedure.read() {
            Procedure::Closure(closure) => object::Procedure::Closure {
                closure: closure.clone(),
                ip: self.ip.get(),
            },
            Procedure::Function(function) => object::Procedure::Function {
                function: function.clone(),
                ip: self.ip.get(),
            },
            Procedure::Native(native) => object::Procedure::Native(native.clone()),
        };

        ObjContinuation::new(
            *self.parent_continuation.read(),
            procedure,
            *self.stack.read(),
            *self.current_input_port.read(),
            *self.current_output_port.read(),
        )
    }

    pub fn apply_continuation(
        &self,
        frame: GcCell<'gc, ObjContinuation<'gc>>,
        mc: MutationContext<'gc, '_>,
    ) {
        let parent_frame = frame.read().frames();
        match frame.read().procedure() {
            object::Procedure::Closure { closure, ip } => {
                *self.procedure.write(mc) = Procedure::Closure(closure.clone());
                self.ip.set(*ip);
            }
            object::Procedure::Function { function, ip } => {
                *self.procedure.write(mc) = Procedure::Function(function.clone());
                self.ip.set(*ip);
            }
            object::Procedure::Native(native) => {
                *self.procedure.write(mc) = Procedure::Native(native.clone());
            }
        }

        *self.parent_continuation.write(mc) = parent_frame;
        let stack = frame.read().stack();
        stack.write(mc).truncate(frame.read().stack_top());
        *self.stack.write(mc) = stack;
        *self.current_input_port.write(mc) = frame.read().current_input_port();
        *self.current_output_port.write(mc) = frame.read().current_output_port();
    }

    /// Core interpreter method that executes bytecode
    pub fn interpret(&self, mc: MutationContext<'gc, '_>) -> Result<()> {
        // Preemptively clone this so we don't hold a borrow on it
        let proc = self.procedure.read().clone();
        let chunk: Gc<'gc, Chunk<'gc>>;
        let environment: Option<Gc<'gc, ObjEnvironment<'gc>>>;
        let stack = *self.stack.read();
        let ip = self.ip.get();
        match proc {
            Procedure::Closure(closure) => {
                chunk = closure.function().chunk();
                environment = Some(closure.environment());
                self.interpret_chunk(mc, chunk, environment, stack, ip)
            }
            Procedure::Function(function) => {
                chunk = function.chunk();
                environment = None;
                self.interpret_chunk(mc, chunk, environment, stack, ip)
            }
            Procedure::Native(native) => {
                let result = native.call(self, stack, mc)?;
                if let Some(result) = result {
                    let frame = *self.parent_continuation.read();
                    if let Some(frame) = frame {
                        self.apply_continuation(frame, mc);
                        self.push_stack(result, mc);
                        Ok(())
                    } else {
                        std::process::exit(0);
                    }
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn current_input_port(&self) -> GcCell<'gc, GcCell<'gc, Object<'gc>>> {
        self.current_input_port
    }

    pub fn current_output_port(&self) -> GcCell<'gc, GcCell<'gc, Object<'gc>>> {
        self.current_output_port
    }

    pub fn parent_continuation(&self) -> GcCell<'gc, Option<GcCell<'gc, ObjContinuation<'gc>>>> {
        self.parent_continuation
    }

    pub(crate) fn procedure(&self) -> GcCell<'gc, Procedure<'gc>> {
        self.procedure
    }

    fn interpret_chunk(
        &self,
        mc: MutationContext<'gc, '_>,
        chunk: Gc<'gc, Chunk<'gc>>,
        environment: Option<Gc<'gc, ObjEnvironment<'gc>>>,
        stack: Stack<'gc>,
        mut ip: usize,
    ) -> Result<()> {
        loop {
            if cfg!(feature = "debug-trace-execution") {
                let stack = stack.read();

                if !stack.is_empty() {
                    print!("          ");
                    for value in stack.iter() {
                        print!("[ {} ]", value);
                    }
                    println!();
                }

                chunk.disassemble_instruction(ip);
            }

            let instruction = OpCode::try_from(read_byte(&chunk, &mut ip)).unwrap();

            match instruction {
                OpCode::ConstantLong => {
                    let constant = read_constant_long(&chunk, &mut ip);
                    stack.write(mc).push(constant);
                }
                OpCode::Constant => {
                    let constant = read_constant(&chunk, &mut ip);
                    stack.write(mc).push(constant);
                }
                OpCode::DefineGlobal => {
                    let name = read_constant(&chunk, &mut ip);
                    let name = name.as_symbol().unwrap();
                    self.define_global(name, peek(stack, 0), mc);
                    stack.write(mc).pop();
                }
                OpCode::GetGlobal => {
                    let name = read_constant(&chunk, &mut ip);
                    let name = name.as_symbol().unwrap();
                    let value = self.globals.read().get(&name).copied();
                    let value = value.ok_or_else(|| {
                        InterpretError::RuntimeError(format!("Undefined variable {}", name))
                    })?;
                    stack.write(mc).push(value);
                }
                OpCode::SetGlobal => {
                    let name = read_constant(&chunk, &mut ip);
                    let name = name.as_symbol().unwrap();
                    if self.globals.read().contains_key(&name) {
                        self.define_global(name, peek(stack, 0), mc);
                    } else {
                        return Err(InterpretError::RuntimeError(format!(
                            "Undefined variable {}",
                            name
                        )));
                    }
                }
                OpCode::GetLocal => {
                    let slot = read_byte(&chunk, &mut ip) as usize;
                    let value = stack.read()[slot];
                    stack.write(mc).push(value);
                }
                OpCode::SetLocal => {
                    let slot = read_byte(&chunk, &mut ip) as usize;
                    stack.write(mc)[slot] = peek(stack, 0);
                }
                OpCode::GetUpvalue => {
                    let slot = read_byte(&chunk, &mut ip) as usize;
                    let value = environment.unwrap().upvalues()[slot].location();
                    stack.write(mc).push(value);
                }
                OpCode::SetUpvalue => {
                    let slot = read_byte(&chunk, &mut ip) as usize;
                    let value = peek(stack, 0);
                    environment.unwrap().upvalues()[slot].set_location(value, mc);
                }
                OpCode::JumpIfFalse => {
                    let offset = read_short(&chunk, &mut ip);
                    if peek(stack, 0).is_falsey() {
                        ip += offset as usize;
                    }
                }
                OpCode::Jump => {
                    let offset = read_short(&chunk, &mut ip);
                    ip += offset as usize;
                }
                OpCode::Call => {
                    let arg_count = read_byte(&chunk, &mut ip);
                    let function = peek(stack, arg_count.into());
                    self.ip.set(ip);
                    self.call_value(function, stack, arg_count as usize, mc)?;
                    return Ok(());
                }
                OpCode::TailCall => {
                    let arg_count = read_byte(&chunk, &mut ip);
                    let function = peek(stack, arg_count.into());
                    self.tail_call_value(function, stack, arg_count as usize, mc)?;
                    return Ok(());
                }
                OpCode::Pop => {
                    stack.write(mc).pop();
                }
                OpCode::Null => stack.write(mc).push(Value::Null),
                OpCode::Void => stack.write(mc).push(Value::Void),
                OpCode::True => stack.write(mc).push(Value::Bool(true)),
                OpCode::False => stack.write(mc).push(Value::Bool(false)),
                OpCode::Closure => {
                    let function = read_constant(&chunk, &mut ip);
                    if let Value::Box(object) = function {
                        if let Object::Function(function) = &*object.read() {
                            let mut upvalues = Vec::new();
                            for _ in 0..function.upvalues().len() {
                                let is_local = read_byte(&chunk, &mut ip);
                                let index = read_byte(&chunk, &mut ip) as usize;
                                if is_local > 0 {
                                    upvalues.push(Upvalue::new(stack, index));
                                } else {
                                    upvalues.push(environment.unwrap().upvalues()[index])
                                }
                            }
                            let environment = Gc::allocate(mc, ObjEnvironment::new(upvalues));
                            let closure = ObjClosure::new(function.clone(), environment);
                            stack
                                .write(mc)
                                .push(Value::boxed(mc, Object::Closure(closure)));
                        } else {
                            return Err(InterpretError::RuntimeError(
                                "Couldn't create closure".to_owned(),
                            ));
                        }
                    } else {
                        return Err(InterpretError::RuntimeError(
                            "Couldn't create closure".to_owned(),
                        ));
                    }
                }
                OpCode::Return => {
                    let result = stack.write(mc).pop().unwrap_or(Value::Void);

                    let frame = *self.parent_continuation.read();
                    if let Some(frame) = frame {
                        self.apply_continuation(frame, mc);
                        self.push_stack(result, mc);
                        return Ok(());
                    } else {
                        std::process::exit(0);
                    }
                }
            }
        }
    }

    pub fn call_value(
        &self,
        callee: Value<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        if let Value::Box(object) = callee {
            match &*object.read() {
                Object::Closure(closure) => self.call_closure(closure, stack, arg_count, mc),
                Object::Continuation(continuation) => {
                    let length = stack.read().len() - arg_count;
                    let mut result = stack.write(mc).split_off(length);
                    self.apply_continuation(GcCell::allocate(mc, continuation.clone()), mc);
                    self.stack.read().write(mc).append(&mut result);
                    Ok(())
                }
                Object::Function(function) => self.call_function(function, stack, arg_count, mc),
                Object::Native(native) => self.call_native(native, stack, arg_count, mc),
                _ => Err(InterpretError::RuntimeError(
                    "Can only call functions".to_string(),
                )),
            }
        } else {
            Err(InterpretError::RuntimeError(
                "Can only call functions".to_string(),
            ))
        }
    }

    fn call_native(
        &self,
        native: &ObjNative<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        let arity = native.arity();
        if !native.is_variadic() && arity != arg_count {
            return Err(InterpretError::RuntimeError(format!(
                "Expected {} arguments but got {}",
                arity, arg_count
            )));
        } else if native.is_variadic() && arity > (arg_count + 1) {
            return Err(InterpretError::RuntimeError(format!(
                "Expected at least {} arguments but got {}",
                arity - 1,
                arg_count
            )));
        }

        // Save current continuation
        let current_continuation = self.save_current_continuation();

        self.parent_continuation
            .write(mc)
            .replace(GcCell::allocate(mc, current_continuation));
        *self.procedure.write(mc) = Procedure::Native(native.clone());
        self.ip.set(0);
        let split = stack.read().len() - arg_count;
        let args = stack.write(mc).split_off(split - 1);
        *self.stack.write(mc) = GcCell::allocate(mc, args);

        Ok(())
    }

    fn call_closure(
        &self,
        closure: &ObjClosure<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        let arity = closure.arity();
        if !closure.is_variadic() && arity != arg_count {
            return Err(InterpretError::RuntimeError(format!(
                "Expected {} arguments but got {}",
                arity, arg_count
            )));
        }

        if closure.is_variadic() {
            let count = arg_count - arity + 1;
            stack.write(mc).push(Value::Null);
            for _ in 0..count {
                let acc = stack.write(mc).pop().unwrap();
                let curr = stack.write(mc).pop().unwrap();
                let pair = ObjPair::new(curr, acc);
                let result = Value::boxed(mc, Object::Pair(pair));
                stack.write(mc).push(result);
            }
        }

        // Save current continuation
        let current_continuation = self.save_current_continuation();

        self.parent_continuation
            .write(mc)
            .replace(GcCell::allocate(mc, current_continuation));
        *self.procedure.write(mc) = Procedure::Closure(closure.clone());
        self.ip.set(0);
        let length = stack.read().len();
        let args = stack.write(mc).split_off(length - arity - 1);
        *self.stack.write(mc) = GcCell::allocate(mc, args);

        Ok(())
    }

    fn call_function(
        &self,
        function: &ObjFunction<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        let arity = function.arity();
        if !function.is_variadic() && arity != arg_count {
            return Err(InterpretError::RuntimeError(format!(
                "Expected {} arguments but got {}",
                arity, arg_count
            )));
        }

        if function.is_variadic() {
            let count = arg_count - arity + 1;
            stack.write(mc).push(Value::Null);
            for _ in 0..count {
                let acc = stack.write(mc).pop().unwrap();
                let curr = stack.write(mc).pop().unwrap();
                let pair = ObjPair::new(curr, acc);
                let result = Value::boxed(mc, Object::Pair(pair));
                stack.write(mc).push(result);
            }
        }

        // Save current continuation
        let current_continuation = self.save_current_continuation();

        self.parent_continuation
            .write(mc)
            .replace(GcCell::allocate(mc, current_continuation));
        *self.procedure.write(mc) = Procedure::Function(function.clone());
        self.ip.set(0);
        let length = stack.read().len();
        let args = stack.write(mc).split_off(length - arity - 1);
        *self.stack.write(mc) = GcCell::allocate(mc, args);

        Ok(())
    }

    pub fn tail_call_value(
        &self,
        callee: Value<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        if let Value::Box(object) = callee {
            match &*object.read() {
                Object::Closure(closure) => self.tail_call_closure(closure, stack, arg_count, mc),
                Object::Continuation(continuation) => {
                    let length = stack.read().len() - arg_count;
                    let mut result = stack.write(mc).split_off(length);
                    self.apply_continuation(GcCell::allocate(mc, continuation.clone()), mc);
                    self.stack.read().write(mc).append(&mut result);
                    Ok(())
                }
                Object::Function(function) => {
                    self.tail_call_function(function, stack, arg_count, mc)
                }
                Object::Native(native) => self.tail_call_native(native, stack, arg_count, mc),
                _ => Err(InterpretError::RuntimeError(
                    "Can only call functions".to_string(),
                )),
            }
        } else {
            Err(InterpretError::RuntimeError(
                "Can only call functions".to_string(),
            ))
        }
    }

    fn tail_call_native(
        &self,
        native: &ObjNative<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        let arity = native.arity();
        if !native.is_variadic() && arity != arg_count {
            return Err(InterpretError::RuntimeError(format!(
                "Expected {} arguments but got {}",
                arity, arg_count
            )));
        } else if native.is_variadic() && arity > (arg_count + 1) {
            return Err(InterpretError::RuntimeError(format!(
                "Expected at least {} arguments but got {}",
                arity - 1,
                arg_count
            )));
        }
        let split = stack.read().len() - arg_count;
        let args = stack.write(mc).split_off(split - 1);
        *self.procedure.write(mc) = Procedure::Native(native.clone());
        *self.stack.write(mc) = GcCell::allocate(mc, args);
        Ok(())
    }

    fn tail_call_function(
        &self,
        function: &ObjFunction<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        let arity = function.arity();
        if !function.is_variadic() && arity != arg_count {
            return Err(InterpretError::RuntimeError(format!(
                "Expected {} arguments but got {}",
                arity, arg_count
            )));
        }

        if function.is_variadic() {
            let count = arg_count - arity + 1;
            stack.write(mc).push(Value::Null);
            for _ in 0..count {
                let acc = stack.write(mc).pop().unwrap();
                let curr = stack.write(mc).pop().unwrap();
                let pair = ObjPair::new(curr, acc);
                let result = Value::boxed(mc, Object::Pair(pair));
                stack.write(mc).push(result);
            }
        }

        *self.procedure.write(mc) = Procedure::Function(function.clone());
        self.ip.set(0);
        let top = stack.read().len();
        let args = stack.write(mc).split_off(top - arity - 1);
        *self.stack.write(mc) = GcCell::allocate(mc, args);

        Ok(())
    }

    fn tail_call_closure(
        &self,
        closure: &ObjClosure<'gc>,
        stack: Stack<'gc>,
        arg_count: usize,
        mc: MutationContext<'gc, '_>,
    ) -> Result<()> {
        let arity = closure.arity();
        if !closure.is_variadic() && arity != arg_count {
            return Err(InterpretError::RuntimeError(format!(
                "Expected {} arguments but got {}",
                arity, arg_count
            )));
        }

        if closure.is_variadic() {
            let count = arg_count - arity + 1;
            stack.write(mc).push(Value::Null);
            for _ in 0..count {
                let acc = stack.write(mc).pop().unwrap();
                let curr = stack.write(mc).pop().unwrap();
                let pair = ObjPair::new(curr, acc);
                let result = Value::boxed(mc, Object::Pair(pair));
                stack.write(mc).push(result);
            }
        }

        *self.procedure.write(mc) = Procedure::Closure(closure.clone());
        self.ip.set(0);
        let top = stack.read().len();
        let args = stack.write(mc).split_off(top - arity - 1);
        *self.stack.write(mc) = GcCell::allocate(mc, args);

        Ok(())
    }

    pub(crate) fn intern_symbol(
        &self,
        token: Token<'gc>,
        mc: MutationContext<'gc, '_>,
    ) -> Symbol<'gc> {
        self.symbol_pool.write(mc).intern(token)
    }

    /// Define a global bindings
    #[inline(always)]
    pub fn define_global(
        &self,
        name: Symbol<'gc>,
        value: Value<'gc>,
        mc: MutationContext<'gc, '_>,
    ) {
        self.globals.write(mc).insert(name, value);
    }

    /// Push a value onto the VM's value stack
    pub(crate) fn push_stack(&self, value: Value<'gc>, mc: MutationContext<'gc, '_>) {
        self.stack.read().write(mc).push(value);
    }
}

/// Peek `distance` from the top of the stack
#[inline(always)]
pub fn peek(stack: Stack<'_>, distance: usize) -> Value<'_> {
    let stack = stack.read();
    stack[stack.len() - distance - 1]
}

/// Read a u8 of data from the chunk at the current IP and update IP
#[inline(always)]
fn read_byte(chunk: &Chunk<'_>, ip: &mut usize) -> u8 {
    let result = chunk.read(*ip);
    *ip += 1;
    result
}

/// Read a u16 of data from the chunk at the current IP and update IP
#[inline(always)]
fn read_short(chunk: &Chunk<'_>, ip: &mut usize) -> u16 {
    ((read_byte(chunk, ip) as u16) << 8) | (read_byte(chunk, ip) as u16)
}

/// Read a constant from the chunk's contant table denoted by the current IP
#[inline(always)]
fn read_constant<'gc>(chunk: &Chunk<'gc>, ip: &mut usize) -> Value<'gc> {
    let offset = read_byte(chunk, ip) as usize;
    chunk.read_constant(offset)
}

/// Read a constant from the chunk's contant table denoted by the current IP
#[inline(always)]
fn read_constant_long<'gc>(chunk: &Chunk<'gc>, ip: &mut usize) -> Value<'gc> {
    // `offset` is a 24-bit uint, but we'll hold it in a u32
    let mut offset: u32 = 0;
    for i in 0..3 {
        let offset_bit = read_byte(chunk, ip) as u32;
        offset |= offset_bit << (8 * i);
    }
    chunk.read_constant(offset as usize)
}
