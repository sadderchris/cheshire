use gc_arena::MutationContext;

use crate::object::{ObjNative, Object, ObjFunction};
use crate::value::Value;
use crate::vm::{Procedure, Result, Stack, VirtualMachine};

pub fn is_procedure<'gc>(
    _: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    _: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    match args[1] {
        Value::Box(object) => Ok(Some(Value::Bool(object.read().is_procedure()))),
        _ => Ok(Some(Value::Bool(false))),
    }
}

pub fn apply<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let mut args = stack.write(mc).pop().unwrap();
    let procedure = stack.read()[1];
    while !args.is_null() {
        let pair = args.as_object()?;
        let pair = pair.read();
        let pair = pair.as_pair()?;
        stack.write(mc).push(pair.car());
        args = pair.cdr();
    }
    let arg_count = stack.read().len() - 2;
    vm.tail_call_value(procedure, stack, arg_count, mc)?;
    Ok(None)
}

pub fn call_with_current_continuation<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let continuation = vm.parent_continuation().read().unwrap().read().clone();
    stack
        .write(mc)
        .push(Value::boxed(mc, Object::Continuation(continuation)));
    let value = stack.read()[1];
    vm.tail_call_value(value, stack, 1, mc)?;
    Ok(None)
}

pub fn values<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let continuation = vm.parent_continuation().read().unwrap().read().clone();
    let arg_count = stack.read().len() - 1;
    vm.tail_call_value(
        Value::boxed(mc, Object::Continuation(continuation)),
        stack,
        arg_count,
        mc,
    )?;
    Ok(None)
}

pub fn call_with_values<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let producer = stack.read()[1];
    // Write the procedure that should pick up execution after this procedure call finishes
    *vm.procedure().write(mc) = Procedure::Native(ObjNative::new(
        2,
        false,
        call_with_values_continuation,
        None,
    ));
    vm.call_value(producer, stack, 0, mc)?;
    Ok(None)
}

fn call_with_values_continuation<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let consumer = stack.read()[2];
    let arg_count = stack.read().len() - 3; // - 2 because of the first two args on the stack from call-with-values
    vm.tail_call_value(consumer, stack, arg_count, mc)?;
    Ok(None)
}

// fn make_procedure<'gc>(
//     vm: &VirtualMachine<'gc>,
//     stack: Stack<'gc>,
//     mc: MutationContext<'gc, '_>,
// ) -> Result<Option<Value<'gc>>> {
//     let proc = ObjFunction::new(mc, arity, variadic, chunk, upvalues, name);
//     let proc = Value::boxed(mc, Object::Function(proc));
//     Ok(Some(proc))
// }
