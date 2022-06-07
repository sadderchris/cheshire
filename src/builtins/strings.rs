use gc_arena::MutationContext;

use crate::memory::{Symbol, Token};
use crate::object::{ObjString, Object};
use crate::value::{TypeError, Value};
use crate::vm::{Result, Stack, VirtualMachine};

pub fn is_string<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    match args[1] {
        Value::String(_) => Ok(Some(Value::Bool(true))),
        Value::Box(object) => Ok(Some(Value::Bool(object.read().is_string()))),
        _ => Ok(Some(Value::Bool(false))),
    }
}

/// Creates an uninterned symbol
pub fn string_to_symbol<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let string = stack.read()[1];
    let symbol = match string {
        Value::String(s) => Symbol::uninterned(Token::from(s)),
        Value::Box(b) => {
            let string = b.read();
            Symbol::uninterned(Token::new(mc, string.as_string()?.clone()))
        }
        _ => return Err(TypeError(format!("'{}' is not a string", string)).into()),
    };

    Ok(Some(Value::Symbol(symbol)))
}

pub fn string_length<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let string = stack.read()[1];
    let length = match string {
        Value::String(s) => s.as_str().chars().count(),
        Value::Box(b) => {
            let string = b.read();
            string.as_string()?.as_str().chars().count()
        }
        _ => return Err(TypeError(format!("'{}' is not a string", string)).into()),
    };

    Ok(Some(Value::Number(length as f64)))
}

pub fn make_string<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let k = args[1].as_number()?;
    let character = if args.len() == 3 {
        args[2].as_char()?
    } else {
        ' '
    };
    let mut buf = vec![0; character.len_utf8()];
    character.encode_utf8(&mut buf);

    let chars: Box<[u8]> = buf
        .into_iter()
        .cycle()
        .take((k as usize) * character.len_utf8())
        .collect();

    Ok(Some(Value::boxed(
        mc,
        Object::String(ObjString::new(chars)),
    )))
}
