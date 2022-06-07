use gc_arena::MutationContext;

use crate::object::{ObjVector, Object};
use crate::value::{TypeError, Value};
use crate::vm::{InterpretError, Result, Stack, VirtualMachine};

pub fn is_vector<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    match args[1] {
        Value::Vector(_) => Ok(Some(Value::Bool(true))),
        Value::Box(object) => Ok(Some(Value::Bool(object.read().is_vector()))),
        _ => Ok(Some(Value::Bool(false))),
    }
}

pub fn make_vector<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    let k = args[1].as_number()?;
    let fill = if args.len() == 3 {
        args[2]
    } else {
        Value::Void
    };

    let buf = vec![fill; k as usize];

    Ok(Some(Value::boxed(
        mc,
        Object::Vector(ObjVector::new(buf.into_boxed_slice())),
    )))
}

pub fn vector_length<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let vector = stack.read()[1];
    let length = match vector {
        Value::Vector(v) => v.as_slice().len(),
        Value::Box(b) => {
            let vector = b.read();
            vector.as_vector()?.as_slice().len()
        }
        _ => return Err(TypeError(format!("'{}' is not a string", vector)).into()),
    };

    Ok(Some(Value::Number(length as f64)))
}

pub fn vector_ref<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let vector = stack.read()[1];
    let offset = stack.read()[2].as_number()? as usize;
    let value = match vector {
        Value::Vector(v) => Value::from(v.as_slice()[offset]),
        Value::Box(b) => {
            let vector = b.read();
            vector.as_vector()?.as_slice()[offset]
        }
        _ => return Err(TypeError(format!("'{}' is not a string", vector)).into()),
    };

    Ok(Some(value))
}

pub fn vector_set<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let vector = stack.read()[1];
    let offset = stack.read()[2].as_number()? as usize;
    let obj = stack.read()[3];
    match vector {
        Value::Vector(_) => {
            return Err(InterpretError::RuntimeError(
                "Expected a mutable vector".into(),
            ))
        }
        Value::Box(b) => {
            let mut vector = b.write(mc);
            vector.as_vector_mut()?.as_slice_mut()[offset] = obj;
        }
        _ => return Err(TypeError(format!("'{}' is not a string", vector)).into()),
    };

    Ok(Some(Value::Void))
}
