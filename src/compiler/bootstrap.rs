use std::borrow::Cow;

use gc_arena::{GcCell, MutationContext};
use thiserror::Error;

use super::{CompilerContext, Upvalue};
use crate::chunk::OpCode;
use crate::memory::Symbol;
use crate::object::{ObjFunction, ObjPair, Object};
use crate::value::{TypeError, Value};

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("[compile]: {0}")]
    Blah(Cow<'static, str>),

    #[error("[compile]: {0}")]
    TypeError(#[from] TypeError),
}

type Result<T> = std::result::Result<T, CompileError>;

fn car(value: Value<'_>) -> Result<Value<'_>> {
    match value {
        Value::Pair(p) => Ok(p.car().into()),
        Value::Box(b) => Ok(b.read().as_pair()?.car()),
        _ => Err(CompileError::TypeError(TypeError(format!(
            "'{}' is not a pair",
            value
        )))),
    }
}

fn cdr(value: Value<'_>) -> Result<Value<'_>> {
    match value {
        Value::Pair(p) => Ok(p.cdr().into()),
        Value::Box(b) => Ok(b.read().as_pair()?.cdr()),
        _ => Err(CompileError::TypeError(TypeError(format!(
            "'{}' is not a pair",
            value
        )))),
    }
}

fn cons<'gc>(car: Value<'gc>, cdr: Value<'gc>, mc: MutationContext<'gc, '_>) -> Result<Value<'gc>> {
    Ok(Value::boxed(mc, Object::Pair(ObjPair::new(car, cdr))))
}

pub fn compile<'gc>(ast: Value<'gc>, mc: MutationContext<'gc, '_>) -> Result<ObjFunction<'gc>> {
    let cc = GcCell::allocate(mc, CompilerContext::default());
    expression(cc, ast, true, None, mc).map_err(|err| {
        print_code(&cc.read());
        err
    })?;

    cc.write(mc).chunk.write(OpCode::Return.into(), 1);
    let (chunk, upvalues) = {
        let cc = cc.read();
        (cc.chunk.clone(), cc.upvalues.clone())
    };

    Ok(ObjFunction::thunk(mc, chunk, upvalues))
}

fn expression<'gc>(
    cc: GcCell<'gc, CompilerContext<'gc>>,
    current: Value<'gc>,
    in_tail_position: bool,
    name: Option<Symbol<'gc>>,
    mc: MutationContext<'gc, '_>,
) -> Result<()> {
    match current {
        Value::Symbol(symbol) => {
            named_variable(&mut cc.write(mc), symbol, false, mc);
            Ok(())
        }
        Value::Pair(_) => definition_or_expression(cc, current, in_tail_position, name, mc),
        Value::Box(b) => match &*b.read() {
            Object::Pair(_) => definition_or_expression(cc, current, in_tail_position, name, mc),
            _ => literal(&mut cc.write(mc), current),
        },
        _ => literal(&mut cc.write(mc), current),
    }
}

fn definition_or_expression<'gc>(
    cc: GcCell<'gc, CompilerContext<'gc>>,
    current: Value<'gc>,
    in_tail_position: bool,
    name: Option<Symbol<'gc>>,
    mc: MutationContext<'gc, '_>,
) -> Result<()> {
    let head = car(current)?;
    let tail = cdr(current)?;
    match head {
        Value::Symbol(s) => match s.as_str().as_ref() {
            "define" => match car(tail)? {
                Value::Symbol(name) => {
                    let expr = car(cdr(tail)?)?;
                    let global = parse_variable(&mut cc.write(mc), name)?;
                    expression(cc, expr, false, Some(name), mc)?;
                    define_variable(&mut cc.write(mc), global as u8, 1);
                    Ok(())
                }
                Value::Pair(formals) => {
                    let name = formals.car().as_symbol()?;
                    let formals = formals.cdr();
                    let bodies = cdr(tail)?;
                    let global = parse_variable(&mut cc.write(mc), name)?;
                    function(cc, formals.into(), bodies, Some(name), false, mc)?;
                    define_variable(&mut cc.write(mc), global as u8, 1);

                    Ok(())
                }
                Value::Box(b) => match &*b.read() {
                    Object::Pair(formals) => {
                        let name = formals.car().as_symbol()?;
                        let formals = formals.cdr();
                        let bodies = cdr(tail)?;
                        let global = parse_variable(&mut cc.write(mc), name)?;
                        function(cc, formals, bodies, Some(name), false, mc)?;
                        define_variable(&mut cc.write(mc), global as u8, 1);

                        Ok(())
                    }
                    _ => Err(CompileError::Blah("Invalid define expression".into())),
                },
                _ => Err(CompileError::Blah("Invalid define expression".into())),
            },
            "set!" => {
                let name = car(tail)?.as_symbol()?;
                let expr = car(cdr(tail)?)?;
                let end = 1;
                expression(cc, expr, false, Some(name), mc)?;
                named_variable(&mut cc.write(mc), name, true, mc);
                cc.write(mc).chunk.write(OpCode::Void.into(), end);
                Ok(())
            }
            "if" => {
                let test = car(tail)?;
                expression(cc, test, false, None, mc)?;
                let then_jump = cc.write(mc).chunk.emit_jump(OpCode::JumpIfFalse, 1);
                cc.write(mc).chunk.write(OpCode::Pop.into(), 1);

                let consequent = car(cdr(tail)?)?;

                expression(cc, consequent, true, None, mc)?;
                let else_jump = cc.write(mc).chunk.emit_jump(OpCode::Jump, 1);
                cc.write(mc).chunk.patch_jump(then_jump);
                cc.write(mc).chunk.write(OpCode::Pop.into(), 1);

                let alternate = cdr(cdr(tail)?)?;
                if !alternate.is_null() {
                    expression(cc, car(alternate)?, true, None, mc)?;
                } else {
                    cc.write(mc).chunk.write(OpCode::Void.into(), 1);
                }
                cc.write(mc).chunk.patch_jump(else_jump);

                Ok(())
            }
            "lambda" => {
                let formals = car(tail)?;
                let bodies = cdr(tail)?;
                function(cc, formals, bodies, name, false, mc)?;

                Ok(())
            }
            "begin" => {
                let line = 1;
                let formals = Value::Null;
                function(cc, formals, tail, None, false, mc)?;

                let opcode = if in_tail_position {
                    OpCode::TailCall
                } else {
                    OpCode::Call
                };

                cc.write(mc).chunk.write(opcode.into(), line);

                cc.write(mc).chunk.write(0, line);

                Ok(())
            }
            "quote" => {
                let lit = car(tail)?;

                literal(&mut cc.write(mc), lit)
            }
            "let" => match car(tail)? {
                Value::Symbol(s) => let_definition(
                    cc,
                    Some(s),
                    car(cdr(tail)?)?,
                    cdr(cdr(tail)?)?,
                    in_tail_position,
                    mc,
                ),
                Value::Pair(_) => {
                    let_definition(cc, None, car(tail)?, cdr(tail)?, in_tail_position, mc)
                }
                Value::Box(b) => match &*b.read() {
                    Object::Pair(_) => {
                        let_definition(cc, None, car(tail)?, cdr(tail)?, in_tail_position, mc)
                    }
                    _ => Err(CompileError::Blah("Invalid let expression".into())),
                },
                _ => Err(CompileError::Blah("Invalid let expression".into())),
            },
            _ => {
                let line = 1;
                named_variable(&mut cc.write(mc), s, false, mc);
                let arg_count = argument_list(cc, tail, mc)?;

                let opcode = if in_tail_position {
                    OpCode::TailCall
                } else {
                    OpCode::Call
                };

                cc.write(mc).chunk.write(opcode.into(), line);

                cc.write(mc).chunk.write(arg_count, line);

                Ok(())
            }
        },
        _ => {
            let line = 1;
            expression(cc, head, false, None, mc)?;
            let arg_count = argument_list(cc, tail, mc)?;

            let opcode = if in_tail_position {
                OpCode::TailCall
            } else {
                OpCode::Call
            };

            cc.write(mc).chunk.write(opcode.into(), line);

            cc.write(mc).chunk.write(arg_count, line);

            Ok(())
        }
    }
}

fn let_definition<'gc>(
    cc: GcCell<'gc, CompilerContext<'gc>>,
    name: Option<Symbol<'gc>>,
    bindings: Value<'gc>,
    bodies: Value<'gc>,
    in_tail_position: bool,
    mc: MutationContext<'gc, '_>,
) -> Result<()> {
    let line = 1;
    let mut curr = bindings;
    let mut formals = Value::Null;
    let mut formals_curr = formals;
    let mut params = Value::Null;
    let mut params_curr = params;
    while !curr.is_null() {
        if formals_curr.is_null() {
            formals_curr = cons(car(car(curr)?)?, Value::Null, mc)?;
            formals = formals_curr;
        } else {
            match &mut *formals_curr.as_object()?.write(mc) {
                Object::Pair(p) => {
                    let formals_next = cons(car(car(curr)?)?, Value::Null, mc)?;
                    p.set_cdr(formals_next);
                    formals_curr = formals_next;
                }
                _ => return Err(CompileError::Blah("Invalid binding list name".into())),
            }
        };
        if params_curr.is_null() {
            params_curr = cons(car(cdr(car(curr)?)?)?, Value::Null, mc)?;
            params = params_curr;
        } else {
            match &mut *params_curr.as_object()?.write(mc) {
                Object::Pair(p) => {
                    let params_next = cons(car(cdr(car(curr)?)?)?, Value::Null, mc)?;
                    p.set_cdr(params_next);
                    params_curr = params_next;
                }
                _ => return Err(CompileError::Blah("Invalid binding list parameter".into())),
            }
        };
        curr = cdr(curr)?;
    }

    function(cc, formals, bodies, name, name.is_some(), mc)?;

    let arg_count = argument_list(cc, params, mc)?;

    let opcode = if in_tail_position {
        OpCode::TailCall
    } else {
        OpCode::Call
    };

    cc.write(mc).chunk.write(opcode.into(), line);

    cc.write(mc).chunk.write(arg_count, line);

    Ok(())
}

fn argument_list<'gc>(
    cc: GcCell<'gc, CompilerContext<'gc>>,
    args: Value<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<u8> {
    let mut arg_count = 0;
    let mut curr = args;
    while !curr.is_null() {
        expression(cc, car(curr)?, false, None, mc)?;

        if arg_count == u8::MAX {
            return Err(CompileError::Blah(
                "Can't have more than 255 arguments".to_string().into(),
            ));
        }
        arg_count += 1;
        curr = cdr(curr)?;
    }

    Ok(arg_count)
}

fn function<'gc>(
    cc: GcCell<'gc, CompilerContext<'gc>>,
    formals: Value<'gc>,
    bodies: Value<'gc>,
    name: Option<Symbol<'gc>>,
    bind_name: bool,
    mc: MutationContext<'gc, '_>,
) -> Result<()> {
    let compiler = GcCell::allocate(mc, CompilerContext::with_parent(cc));

    if bind_name {
        compiler.write(mc).local0 = name;
    }

    let (arity, variadic) = parse_formals(&mut compiler.write(mc), formals)?;

    let last_line = parse_bodies(compiler, bodies, mc)?;

    let object = Object::Function(ObjFunction::new(
        mc,
        arity as usize,
        variadic,
        compiler.read().chunk.clone(),
        compiler.read().upvalues.clone(),
        name,
    ));

    let value = Value::boxed(mc, object);

    if !compiler.read().upvalues.is_empty() {
        cc.write(mc).chunk.write(OpCode::Closure.into(), last_line);

        let offset = cc.write(mc).chunk.add_constant(value);

        cc.write(mc).chunk.write(offset as u8, last_line);

        for upvalue in compiler.read().upvalues.iter() {
            let is_local = if upvalue.is_local { 1 } else { 0 };
            cc.write(mc).chunk.write(is_local, last_line);
            cc.write(mc).chunk.write(upvalue.index, last_line);
        }
    } else {
        cc.write(mc).chunk.write_constant(value, last_line);
    }

    Ok(())
}

fn parse_formals<'gc>(cc: &mut CompilerContext<'gc>, formals: Value<'gc>) -> Result<(u8, bool)> {
    match formals {
        Value::Pair(p) => {
            let formal = p.car();

            // let line = formal.as_span().start_pos().line_col().0;
            let param_constant = parse_variable(cc, formal.as_symbol()?)?;
            define_variable(cc, param_constant as u8, 1);

            let (mut arity, variadic) = parse_formals(cc, p.cdr().into())?;
            arity += 1;
            if arity == u8::MAX {
                return Err(CompileError::Blah(
                    "Can't have more than 255 parameters".into(),
                ));
            }
            Ok((arity, variadic))
        }
        Value::Box(b) => {
            match &*b.read() {
                Object::Pair(p) => {
                    let formal = p.car();

                    // let line = formal.as_span().start_pos().line_col().0;
                    let param_constant = parse_variable(cc, formal.as_symbol()?)?;
                    define_variable(cc, param_constant as u8, 1);

                    let (mut arity, variadic) = parse_formals(cc, p.cdr())?;
                    arity += 1;
                    if arity == u8::MAX {
                        return Err(CompileError::Blah(
                            "Can't have more than 255 parameters".into(),
                        ));
                    }
                    Ok((arity, variadic))
                }
                _ => Err(CompileError::Blah("Malformed formals".into())),
            }
        }
        Value::Symbol(s) => {
            // let line = formals.as_span().start_pos().line_col().0;
            let param_constant = parse_variable(cc, s)?;
            define_variable(cc, param_constant as u8, 1);
            Ok((1, true))
        }
        Value::Null => Ok((0, false)),
        _ => Err(CompileError::Blah("Malformed formals".into())),
    }
}

fn parse_bodies<'gc>(
    cc: GcCell<'gc, CompilerContext<'gc>>,
    mut remaining_bodies: Value<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Result<usize> {
    // let mut last_line = body.as_span().end_pos().line_col().0;
    let mut last_line = 1;
    let mut in_tail_position = false;

    while !in_tail_position {
        // last_line = body.as_span().end_pos().line_col().0;
        last_line = 1;
        let body =
            car(remaining_bodies).map_err(|_| CompileError::Blah("Invalid bodies list".into()))?;
        remaining_bodies =
            cdr(remaining_bodies).map_err(|_| CompileError::Blah("Invalid bodies list".into()))?;
        in_tail_position = remaining_bodies.is_null();
        expression(cc, body, in_tail_position, None, mc)?;
    }

    cc.write(mc).chunk.write(OpCode::Return.into(), last_line);

    Ok(last_line)
}

fn parse_variable<'gc>(cc: &mut CompilerContext<'gc>, name: Symbol<'gc>) -> Result<usize> {
    declare_variable(cc, name)?;
    if cc.scope_depth > 0 {
        Ok(0)
    } else {
        Ok(make_symbol(cc, name))
    }
}

fn declare_variable<'gc>(cc: &mut CompilerContext<'gc>, name: Symbol<'gc>) -> Result<()> {
    if cc.scope_depth == 0 {
        return Ok(());
    }

    add_local(cc, name)
}

fn literal<'gc>(cc: &mut CompilerContext<'gc>, result: Value<'gc>) -> Result<()> {
    // let line = span.start_pos().line_col().0;
    let line = 1;
    match result {
        Value::Bool(b) => {
            let opcode = if b { OpCode::True } else { OpCode::False };

            cc.chunk.write(opcode.into(), line)
        }
        Value::Null => cc.chunk.write(OpCode::Null.into(), line),
        Value::Void => cc.chunk.write(OpCode::Void.into(), line),
        _ => cc.chunk.write_constant(result, line),
    }

    Ok(())
}

fn resolve_local<'gc>(cc: &CompilerContext<'gc>, name: Symbol<'gc>) -> Option<usize> {
    if let Some(local0_name) = cc.local0 {
        if local0_name == name {
            return Some(0);
        }
    }

    cc.locals.get_index_of(&name).map(|val| val + 1)
}

fn named_variable<'gc>(
    cc: &mut CompilerContext<'gc>,
    symbol: Symbol<'gc>,
    is_assign: bool,
    mc: MutationContext<'gc, '_>,
) {
    let (arg, get_op, set_op) = {
        let arg = resolve_local(cc, symbol);
        if let Some(arg) = arg {
            (arg, OpCode::GetLocal, OpCode::SetLocal)
        } else if let Some(arg) = resolve_upvalue(cc, symbol, mc) {
            (arg, OpCode::GetUpvalue, OpCode::SetUpvalue)
        } else {
            (
                cc.chunk.add_constant(Value::Symbol(symbol)),
                OpCode::GetGlobal,
                OpCode::SetGlobal,
            )
        }
    };

    let opcode = if is_assign { set_op } else { get_op };

    cc.chunk.write(opcode.into(), 1);
    cc.chunk.write(arg as u8, 1);
}

fn resolve_upvalue<'gc>(
    cc: &mut CompilerContext<'gc>,
    name: Symbol<'gc>,
    mc: MutationContext<'gc, '_>,
) -> Option<usize> {
    let local = resolve_local(&cc.parent?.read(), name).map(|local| {
        cc.upvalues.add_upvalue(Upvalue {
            index: local as u8,
            is_local: true,
        })
    });

    if local.is_some() {
        return local;
    }

    resolve_upvalue(&mut cc.parent?.write(mc), name, mc).map(|upvalue| {
        cc.upvalues.add_upvalue(Upvalue {
            index: upvalue as u8,
            is_local: false,
        })
    })
}

fn add_local<'gc>(cc: &mut CompilerContext<'gc>, name: Symbol<'gc>) -> Result<()> {
    if cc.locals.len() > u8::MAX as usize + 1 {
        return Err(CompileError::Blah(
            "Too many local variables in function".into(),
        ));
    }

    if cc.locals.contains(&name) {
        return Err(CompileError::Blah(
            format!("Already variable with the name {} in this scope", name).into(),
        ));
    }

    cc.locals.insert(name);
    Ok(())
}

fn define_variable(cc: &mut CompilerContext<'_>, global: u8, line: usize) {
    if cc.scope_depth > 0 {
        return;
    }

    cc.chunk.write(OpCode::DefineGlobal.into(), line);
    cc.chunk.write(global, line);
    cc.chunk.write(OpCode::Void.into(), line); // In case this is the last thing in the chunk
}

fn make_symbol<'gc>(cc: &mut CompilerContext<'gc>, name: Symbol<'gc>) -> usize {
    cc.chunk.add_constant(Value::Symbol(name))
}

#[inline(always)]
fn print_code(cc: &CompilerContext<'_>) {
    if cfg!(features = "debug-print-code") {
        cc.chunk.disassemble("<script>");
    }
}
