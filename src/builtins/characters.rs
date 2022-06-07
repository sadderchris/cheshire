use gc_arena::MutationContext;

use crate::value::{Char, Value};
use crate::vm::{Result, Stack, VirtualMachine};

pub fn is_char<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    Ok(Some(Value::Bool(args[1].is_char())))
}

pub fn is_char_eq<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c1 = args[1].as_char()?;
    let c2 = args[2].as_char()?;
    Ok(Some(Value::Bool(c1 == c2)))
}

pub fn is_char_lt<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c1 = args[1].as_char()?;
    let c2 = args[2].as_char()?;
    Ok(Some(Value::Bool(c1 < c2)))
}

pub fn is_char_gt<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c1 = args[1].as_char()?;
    let c2 = args[2].as_char()?;
    Ok(Some(Value::Bool(c1 > c2)))
}

pub fn is_char_lte<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c1 = args[1].as_char()?;
    let c2 = args[2].as_char()?;
    Ok(Some(Value::Bool(c1 <= c2)))
}

pub fn is_char_gte<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c1 = args[1].as_char()?;
    let c2 = args[2].as_char()?;
    Ok(Some(Value::Bool(c1 >= c2)))
}

pub fn is_char_alphabetic<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c = args[1].as_char()?;
    Ok(Some(Value::Bool(c.is_alphabetic())))
}

pub fn is_char_numeric<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c = args[1].as_char()?;
    Ok(Some(Value::Bool(c.is_numeric())))
}

pub fn is_char_whitespace<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c = args[1].as_char()?;
    Ok(Some(Value::Bool(c.is_whitespace())))
}

pub fn is_char_upper_case<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c = args[1].as_char()?;
    Ok(Some(Value::Bool(c.is_uppercase())))
}

pub fn is_char_lower_case<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c = args[1].as_char()?;
    Ok(Some(Value::Bool(c.is_lowercase())))
}

pub fn char_upcase<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c = args[1].as_char()?;
    Ok(Some(Value::Char(Char(c.to_ascii_uppercase()))))
}

pub fn char_downcase<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let c = args[1].as_char()?;
    Ok(Some(Value::Char(Char(c.to_ascii_lowercase()))))
}
