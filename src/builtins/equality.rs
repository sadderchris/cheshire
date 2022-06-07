use gc_arena::{Gc, GcCell, MutationContext};

use crate::value::Value;
use crate::vm::{Result, Stack, VirtualMachine};

pub fn is_eqv<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    use Value::*;
    match (args[1], args[2]) {
        (Bool(b1), Bool(b2)) => Ok(Some(Bool(b1 == b2))),
        (Char(c1), Char(c2)) => Ok(Some(Bool(c1 == c2))),
        (Number(_), Number(_)) => super::equal_number(vm, stack, mc),
        (Null, Null) => Ok(Some(Bool(true))),
        (Pair(pair1), Pair(pair2)) => Ok(Some(Bool(Gc::ptr_eq(pair1, pair2)))),
        (String(string1), String(string2)) => Ok(Some(Bool(Gc::ptr_eq(string1, string2)))),
        (Box(obj1), Box(obj2)) => (Ok(Some(Bool(GcCell::ptr_eq(obj1, obj2))))),
        (Symbol(s1), Symbol(s2)) => Ok(Some(Bool(s1 == s2))),
        (Void, Void) => Ok(Some(Bool(true))),
        (_, _) => Ok(Some(Bool(false))),
    }
}

pub fn is_eq<'gc>(
    vm: &VirtualMachine<'gc>,
    stack: Stack<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<Option<Value<'gc>>> {
    let args = stack.read();
    match (args[1], args[2]) {
        (Value::Number(n1), Value::Number(n2)) => Ok(Some(Value::Bool(n1 == n2))),
        (_, _) => is_eqv(vm, stack, mc),
    }
}
