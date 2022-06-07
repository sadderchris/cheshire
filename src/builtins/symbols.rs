use std::ops::Deref;

use gc_arena::{GcCell, MutationContext};

use crate::object::Object;
use crate::value::Value;
use crate::vm::{Result, Stack, VirtualMachine};

pub fn symbol_to_string<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let symbol = stack.read()[1].as_symbol()?;
    Ok(Some(Value::Box(GcCell::allocate(
        mc,
        Object::String(symbol.deref().clone()),
    ))))
}

pub fn is_symbol<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    Ok(Some(Value::Bool(args[1].is_symbol())))
}
