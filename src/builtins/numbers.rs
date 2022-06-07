use gc_arena::MutationContext;

use crate::value::Value;
use crate::vm::{InterpretError, Result, Stack, VirtualMachine};

pub fn plus<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    plus_impl(&args[1..])
}

fn plus_impl<'gc>(args: &[Value<'gc>]) -> Result<Option<Value<'gc>>> {
    let mut result = 0f64;
    for arg in args.iter() {
        let arg = arg.as_number()?;
        result += arg;
    }
    Ok(Some(Value::Number(result)))
}

pub fn minus<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    if stack.read().len() == 2 {
        Ok(Some(Value::Number(-stack.read()[1].as_number()?)))
    } else {
        let args = stack.write(mc).split_off(2);
        let result = stack.read()[1].as_number()?
            - plus_impl(&args)?
                .ok_or_else(|| InterpretError::RuntimeError("Shouldn't get here".to_string()))?
                .as_number()?;
        Ok(Some(Value::Number(result)))
    }
}

pub fn multiply<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    multiply_impl(&args[1..])
}

fn multiply_impl<'gc>(args: &[Value<'gc>]) -> Result<Option<Value<'gc>>> {
    let mut result = 1f64;
    for arg in args.iter() {
        let arg = arg.as_number()?;
        result *= arg;
    }
    Ok(Some(Value::Number(result)))
}

pub fn divide<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    if stack.read().len() == 2 {
        let args = stack.read();
        Ok(Some(Value::Number(1f64 / args[1].as_number()?)))
    } else {
        let args = stack.write(mc).split_off(2);
        let result = stack.read()[1].as_number()?
            / multiply_impl(&args)?
                .ok_or_else(|| InterpretError::RuntimeError("None received?".to_string()))?
                .as_number()?;
        Ok(Some(Value::Number(result)))
    }
}

pub fn is_number<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    Ok(Some(Value::Bool(args[1].is_number())))
}

pub fn equal_number<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let mut first = args[1].as_number()?;
    for second in &args[2..] {
        if (first - second.as_number()?).abs() > f64::EPSILON {
            return Ok(Some(Value::Bool(false)));
        }

        first = second.as_number()?;
    }

    Ok(Some(Value::Bool(true)))
}

pub fn lt_number<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let mut first = args[1].as_number()?;
    for second in &args[2..] {
        if first >= second.as_number()? {
            return Ok(Some(Value::Bool(false)));
        }

        first = second.as_number()?;
    }

    Ok(Some(Value::Bool(true)))
}

pub fn gt_number<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let mut first = args[1].as_number()?;
    for second in &args[2..] {
        if first <= second.as_number()? {
            return Ok(Some(Value::Bool(false)));
        }

        first = second.as_number()?;
    }

    Ok(Some(Value::Bool(true)))
}

pub fn lte_number<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let mut first = args[1].as_number()?;
    for second in &args[2..] {
        if first > second.as_number()? {
            return Ok(Some(Value::Bool(false)));
        }

        first = second.as_number()?;
    }

    Ok(Some(Value::Bool(true)))
}

pub fn gte_number<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let mut first = args[1].as_number()?;
    for second in &args[2..] {
        if first < second.as_number()? {
            return Ok(Some(Value::Bool(false)));
        }

        first = second.as_number()?;
    }

    Ok(Some(Value::Bool(true)))
}
