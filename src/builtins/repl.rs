use std::fs::File;
use std::io::{self, Write};
use std::ops::Deref;

use gc_arena::MutationContext;

use crate::compiler::bootstrap;
use crate::memory::{Symbol, Token};
use crate::object::{ObjNative, ObjReadPort, ObjString, Object};
use crate::value::Value;
use crate::vm::{peek, InterpretError, Procedure, Result, Stack, VirtualMachine};

pub fn read_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let is_char_ready = vm
        .current_input_port()
        .read()
        .read()
        .as_read_port()?
        .is_char_ready();
    if !is_char_ready {
        print!(">> ");
        let _ = io::stdout().flush();
    }

    // Write the procedure that should pick up execution after this procedure call finishes
    *vm.procedure().write(mc) = Procedure::Native(ObjNative::new(1, false, compile_thunk, None));

    let read = Value::boxed(
        mc,
        Object::Native(ObjNative::new(
            0,
            true,
            super::read_input,
            Some(Symbol::uninterned(Token::new(mc, ObjString::from("read")))),
        )),
    );
    stack.write(mc).push(read);

    vm.call_value(read, stack, 0, mc)?;
    Ok(None)
}

fn car(list: Value<'_>) -> Option<Value<'_>> {
    match list {
        Value::Pair(pair) => Some(pair.car().into()),
        Value::Box(obj) => match &*obj.read() {
            Object::Pair(pair) => Some(pair.car()),
            _ => None,
        },
        _ => None,
    }
}

fn compile_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let result = peek(stack, 0);
    if result.is_null() {
        let repl = Value::boxed(
            mc,
            Object::Native(ObjNative::new(0, false, read_thunk, None)),
        );
        stack.write(mc).push(repl);

        vm.tail_call_value(repl, stack, 0, mc)?;
        return Ok(None);
    }

    let result = car(result).unwrap();
    if result.is_eof() {
        let frame = *vm.parent_continuation().read();
        if let Some(frame) = frame {
            vm.apply_continuation(frame, mc);
            vm.push_stack(result, mc);
            return Ok(None);
        } else {
            std::process::exit(0);
        }
    }

    // Write the procedure that should pick up execution after this procedure call finishes
    *vm.procedure().write(mc) = Procedure::Native(ObjNative::new(1, false, eval_thunk, None));

    let compile = Value::boxed(
        mc,
        Object::Native(ObjNative::new(
            1,
            false,
            compile,
            Some(Symbol::uninterned(Token::new(
                mc,
                ObjString::from("compile"),
            ))),
        )),
    );
    stack.write(mc).push(compile);
    stack.write(mc).push(result);

    vm.call_value(compile, stack, 1, mc)?;
    Ok(None)
}

fn eval_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let eval = peek(stack, 0);

    // Write the procedure that should pick up execution after this procedure call finishes
    *vm.procedure().write(mc) = Procedure::Native(ObjNative::new(1, false, print_thunk, None));

    vm.call_value(eval, stack, 0, mc)?;
    Ok(None)
}

fn print_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let result = stack.write(mc).pop().unwrap();
    if !result.is_void() && !result.is_eof() {
        println!("{}", result);
    }

    let repl = Value::boxed(
        mc,
        Object::Native(ObjNative::new(0, false, read_thunk, None)),
    );
    stack.write(mc).push(repl);

    vm.tail_call_value(repl, stack, 0, mc)?;
    Ok(None)
}

pub fn compile<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let value = stack.read()[1];
    let result = bootstrap::compile(value, mc)?;
    Ok(Some(Value::boxed(mc, Object::Function(result))))
}

pub fn load<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let file_name = stack.write(mc).pop().unwrap();
    let file_name = match file_name {
        Value::String(s) => s.deref().clone(),
        Value::Box(b) => match &*b.read() {
            Object::String(s) => s.clone(),
            _ => return Err(InterpretError::RuntimeError("Expected string".into())),
        },
        _ => return Err(InterpretError::RuntimeError("Expected string".into())),
    };

    let loader = Value::boxed(
        mc,
        Object::Native(ObjNative::new(1, false, load_read_thunk, None)),
    );
    stack.write(mc).push(loader);

    let reader = Value::boxed(
        mc,
        Object::ReadPort(ObjReadPort::new(File::open(file_name.as_str().as_ref())?)),
    );
    stack.write(mc).push(reader);

    vm.tail_call_value(loader, stack, 1, mc)?;
    Ok(None)
}

fn load_read_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let top = peek(stack, 0);

    // Write the procedure that should pick up execution after this procedure call finishes
    *vm.procedure().write(mc) =
        Procedure::Native(ObjNative::new(1, false, load_compile_thunk, None));

    let read = Value::boxed(
        mc,
        Object::Native(ObjNative::new(
            1,
            true,
            super::read_input,
            Some(Symbol::uninterned(Token::new(mc, ObjString::from("read")))),
        )),
    );
    stack.write(mc).push(read);
    stack.write(mc).push(top);

    vm.call_value(read, stack, 1, mc)?;
    Ok(None)
}

fn load_compile_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let result = peek(stack, 0);
    if result.is_null() {
        let loader = Value::boxed(
            mc,
            Object::Native(ObjNative::new(1, false, load_read_thunk, None)),
        );
        let reader = stack.read()[1];
        stack.write(mc).push(loader);
        stack.write(mc).push(reader);

        vm.tail_call_value(loader, stack, 1, mc)?;
        return Ok(None);
    }

    let result = car(result).unwrap();
    if result.is_eof() {
        let frame = *vm.parent_continuation().read();
        if let Some(frame) = frame {
            vm.apply_continuation(frame, mc);
            vm.push_stack(result, mc);
            return Ok(None);
        } else {
            std::process::exit(0);
        }
    }

    // Write the procedure that should pick up execution after this procedure call finishes
    *vm.procedure().write(mc) = Procedure::Native(ObjNative::new(1, false, load_eval_thunk, None));

    let compile = Value::boxed(
        mc,
        Object::Native(ObjNative::new(
            1,
            false,
            compile,
            Some(Symbol::uninterned(Token::new(
                mc,
                ObjString::from("compile"),
            ))),
        )),
    );
    stack.write(mc).push(compile);
    stack.write(mc).push(result);

    vm.call_value(compile, stack, 1, mc)?;
    Ok(None)
}

fn load_eval_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let eval = peek(stack, 0);

    // Write the procedure that should pick up execution after this procedure call finishes
    *vm.procedure().write(mc) =
        Procedure::Native(ObjNative::new(2, false, load_eval_continuation_thunk, None));

    vm.call_value(eval, stack, 0, mc)?;
    Ok(None)
}

fn load_eval_continuation_thunk<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let _ = stack.write(mc).pop();
    let reader = stack.read()[1];

    // Write the procedure that should pick up execution after this procedure call finishes
    let loader = Value::boxed(
        mc,
        Object::Native(ObjNative::new(1, false, load_read_thunk, None)),
    );
    stack.write(mc).push(loader);
    stack.write(mc).push(reader);

    vm.tail_call_value(loader, stack, 1, mc)?;
    Ok(None)
}

pub fn exit<'gc>(
    _: &VirtualMachine<'gc>,
    _: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    // let exit_code = stack.write(mc).pop()
    //     .map(|v| v.as_number().map(|num| num as i32))
    //     .unwrap_or(Ok(0))?;
    std::process::exit(0);
}

pub fn disassemble<'gc>(
    _: &VirtualMachine<'gc>,
    args: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let func = args.read()[1];
    let (chunk, name) = match &*func.as_object()?.read() {
        Object::Function(f) => (f.chunk(), f.name()),
        Object::Closure(c) => (c.function().chunk(), c.function().name()),
        _ => {
            return Err(InterpretError::RuntimeError(
                "Argument must be a function!".into(),
            ))
        }
    };

    let name = name
        .as_ref()
        .map(|sym| sym.as_str())
        .unwrap_or_else(|| "anonymous procedure".into());
    chunk.disassemble(&name);
    Ok(Some(Value::Void))
}
