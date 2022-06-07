use gc_arena::MutationContext;
use pest::Parser;

use crate::compiler;
use crate::object::{ObjReadPort, ObjPair, Object};
use crate::scanner::{Rule, SchemeParser};
use crate::value::{Char, Value};
use crate::vm::{InterpretError, Result, Stack, VirtualMachine};

pub fn is_input_port<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    Ok(Some(Value::Bool(
        stack.read()[1].as_object()?.read().as_read_port().is_ok(),
    )))
}

pub fn is_output_port<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    Ok(Some(Value::Bool(
        stack.read()[1].as_object()?.read().as_write_port().is_ok(),
    )))
}

pub fn current_input_port<'gc>(
    vm: &VirtualMachine<'gc>,
    _: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    Ok(Some(Value::Box(*vm.current_input_port().read())))
}

pub fn current_output_port<'gc>(
    vm: &VirtualMachine<'gc>,
    _: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    Ok(Some(Value::Box(*vm.current_output_port().read())))
}

pub fn is_char_ready<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let len = stack.read().len() - 1;
    let result = match len {
        0 => vm
            .current_input_port()
            .read()
            .read()
            .as_read_port()?
            .is_char_ready(),
        1 => stack.read()[1]
            .as_object()?
            .read()
            .as_read_port()?
            .is_char_ready(),
        _ => {
            return Err(InterpretError::RuntimeError(format!(
                "Expected 0 or 1 arguments, but received {}",
                len
            )))
        }
    };

    Ok(Some(Value::Bool(result)))
}

pub fn read_char<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let len = stack.read().len() - 1;
    let result = match len {
        0 => vm
            .current_input_port()
            .read()
            .write(mc)
            .as_read_port_mut()?
            .read_char()?,
        1 => stack.read()[1]
            .as_object()?
            .write(mc)
            .as_read_port_mut()?
            .read_char()?,
        _ => {
            return Err(InterpretError::RuntimeError(format!(
                "Expected 0 or 1 arguments, but received {}",
                len
            )))
        }
    };

    let result = match result {
        Some(character) => Value::Char(Char(character)),
        None => Value::Eof,
    };

    Ok(Some(result))
}

pub fn peek_char<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let len = stack.read().len() - 1;
    let result = match len {
        0 => vm
            .current_input_port()
            .read()
            .write(mc)
            .as_read_port_mut()?
            .peek_char()?,
        1 => stack.read()[1]
            .as_object()?
            .write(mc)
            .as_read_port_mut()?
            .peek_char()?,
        _ => {
            return Err(InterpretError::RuntimeError(format!(
                "Expected 0 or 1 arguments, but received {}",
                len
            )))
        }
    };

    let result = match result {
        Some(character) => Value::Char(Char(character)),
        None => Value::Eof,
    };

    Ok(Some(result))
}

pub fn write_char<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let len = args.len() - 1;
    let character = args[1].as_char()?;

    let _ = match len {
        1 => vm
            .current_output_port()
            .read()
            .write(mc)
            .as_write_port_mut()?
            .write_char(character)?,

        2 => stack.read()[2]
            .as_object()?
            .write(mc)
            .as_write_port_mut()?
            .write_char(character)?,
        _ => {
            return Err(InterpretError::RuntimeError(format!(
                "Expected 1 or 2 arguments, but received {}",
                len
            )))
        }
    };

    Ok(Some(Value::Void))
}

pub fn read<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let port = if args.len() == 1 {
        *vm.current_input_port().read()
    } else if args.len() == 2 {
        args[1].as_object()?
    } else {
        return Err(InterpretError::RuntimeError(format!(
            "Expected 0 or 1 arguments, but received {}",
            args.len()
        )));
    };

    let mut port = port.write(mc);
    let port = port.as_read_port_mut()?;
    let (result, consumed) = match read_from_port(vm, port, mc) {
        Ok((None, consumed)) => (Ok(Some(Value::Eof)), consumed),
        Ok((value, consumed)) => (Ok(value), consumed),
        Err((err, consumed)) => (Err(err), consumed),
    };
    port.consume(consumed);
    result
}

pub fn read_input<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let port = if args.len() == 1 {
        *vm.current_input_port().read()
    } else if args.len() == 2 {
        args[1].as_object()?
    } else {
        return Err(InterpretError::RuntimeError(format!(
            "Expected 0 or 1 arguments, but received {}",
            args.len()
        )));
    };

    let mut port = port.write(mc);
    let port = port.as_read_port_mut()?;
    let (result, consumed) = match read_from_port(vm, port, mc) {
        Ok((None, consumed)) => (Ok(Some(Value::Null)), consumed),
        Ok((Some(value), consumed)) => {
            let result = Value::boxed(
                mc,
                Object::Pair(ObjPair::new(value, Value::Null)),
            );
            (Ok(Some(result)), consumed)
        }
        Err((err, consumed)) => (Err(err), consumed),
    };
    port.consume(consumed);
    result
}

fn read_from_port<'gc>(
    vm: &VirtualMachine<'gc>,
    input_port: &mut ObjReadPort,
    mc: MutationContext<'gc, '_>,
) -> std::result::Result<(Option<Value<'gc>>, usize), (InterpretError, usize)> {
    let buf = input_port
        .fill_buf()
        .map_err(|e| (InterpretError::from(e), 0))?;
    let orig_len = buf.len();
    let orig_source = core::str::from_utf8(buf).map_err(|e| (InterpretError::from(e), orig_len))?;
    let source = orig_source.trim_start();
    let white_len = orig_len - source.len();

    if white_len > 0 && white_len == orig_len {
        return Ok((None, white_len));
    }

    let mut pairs = SchemeParser::parse(Rule::repl, source)
        .map_err(|e| (InterpretError::from(e), orig_len))?;
    let pair = pairs.next();
    if pair.is_none() {
        return Ok((None, orig_len));
    }

    let pair = pair.unwrap();
    let len = pair.as_span().end();
    let expr = compiler::read(pair, vm, mc).map_err(|e| (InterpretError::from(e), orig_len))?;
    let result = if orig_source[(len + white_len)..].trim_start().is_empty() {
        (Some(expr.into_boxed_value(mc)), orig_len)
    } else {
        (Some(expr.into_boxed_value(mc)), len + white_len)
    };

    Ok(result)
}

pub fn is_eof_object<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    Ok(Some(Value::Bool(args[1].is_eof())))
}
