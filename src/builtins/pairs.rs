use gc_arena::MutationContext;

use crate::object::{ObjPair, Object};
use crate::value::Value;
use crate::vm::{InterpretError, Result, Stack, VirtualMachine};

pub fn cons<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    Ok(Some(Value::boxed(
        mc,
        Object::Pair(ObjPair::new(args[1], args[2])),
    )))
}

pub fn car<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    match args[1] {
        Value::Pair(pair) => Ok(Some(pair.car().into())),
        Value::Box(object) => Ok(Some(object.read().as_pair()?.car())),
        _ => Err(InterpretError::RuntimeError(format!(
            "{} is not a pair",
            args[1]
        ))),
    }
}

pub fn set_car<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    args[1]
        .as_object()?
        .write(mc)
        .as_pair_mut()?
        .set_car(args[2]);
    Ok(Some(Value::Void))
}

pub fn cdr<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    match args[1] {
        Value::Pair(pair) => Ok(Some(pair.cdr().into())),
        Value::Box(object) => Ok(Some(object.read().as_pair()?.cdr())),
        _ => Err(InterpretError::RuntimeError(format!(
            "{} is not a pair",
            args[1]
        ))),
    }
}

pub fn set_cdr<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    args[1]
        .as_object()?
        .write(mc)
        .as_pair_mut()?
        .set_cdr(args[2]);
    Ok(Some(Value::Void))
}

pub fn is_pair<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    match args[1] {
        Value::Pair(_) => Ok(Some(Value::Bool(true))),
        Value::Box(object) => Ok(Some(Value::Bool(object.read().is_pair()))),
        _ => Ok(Some(Value::Bool(false))),
    }
}
