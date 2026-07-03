use crate::engine::errors::EngineError;
use crate::engine::functions::FunctionDef;
use crate::engine::types::Number;
use num::Zero;

pub fn is_truthy(n: &Number) -> bool {
    // Zero (and NaN) is false, anything else is true
    match n {
        Number::Integer(i) => !i.is_zero(),
        Number::Rational(r) => !r.is_zero(),
        Number::Float(f) => *f != 0.0 && !f.is_nan(),
        Number::Complex(c) => !c.is_zero(),
    }
}

fn from_bool(b: bool) -> Number {
    if b {
        Number::Integer(1.into())
    } else {
        Number::Integer(0.into())
    }
}

pub fn true_val(_args: &[Number]) -> Result<Number, EngineError> {
    Ok(from_bool(true))
}

pub fn false_val(_args: &[Number]) -> Result<Number, EngineError> {
    Ok(from_bool(false))
}

pub fn not(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::arity("not", 1, args.len()));
    }
    Ok(from_bool(!is_truthy(&args[0])))
}

pub fn and(args: &[Number]) -> Result<Number, EngineError> {
    for arg in args {
        if !is_truthy(arg) {
            return Ok(from_bool(false));
        }
    }
    Ok(from_bool(true))
}

pub fn or(args: &[Number]) -> Result<Number, EngineError> {
    for arg in args {
        if is_truthy(arg) {
            return Ok(from_bool(true));
        }
    }
    Ok(from_bool(false))
}

pub fn xor(args: &[Number]) -> Result<Number, EngineError> {
    let mut true_count = 0;
    for arg in args {
        if is_truthy(arg) {
            true_count += 1;
        }
    }
    // XOR is true if an odd number of arguments are true
    Ok(from_bool(true_count % 2 != 0))
}

pub fn if_func(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 3 {
        return Err(EngineError::arity("if", 3, args.len()));
    }
    // args[0] is condition, args[1] is then, args[2] is else.
    // Note: this eager form is only a fallback. Expr::eval special-cases `if` so
    // that normally only the taken branch is evaluated (see ast.rs).
    if is_truthy(&args[0]) {
        Ok(args[1].clone())
    } else {
        Ok(args[2].clone())
    }
}

inventory::submit! { FunctionDef { name: "TRUE", func: true_val } }
inventory::submit! { FunctionDef { name: "FALSE", func: false_val } }
inventory::submit! { FunctionDef { name: "NOT", func: not } }
inventory::submit! { FunctionDef { name: "AND", func: and } }
inventory::submit! { FunctionDef { name: "OR", func: or } }
inventory::submit! { FunctionDef { name: "XOR", func: xor } }

// Support lowercase alias as well for common usage
inventory::submit! { FunctionDef { name: "true", func: true_val } }
inventory::submit! { FunctionDef { name: "false", func: false_val } }
inventory::submit! { FunctionDef { name: "not", func: not } }
inventory::submit! { FunctionDef { name: "and", func: and } }
inventory::submit! { FunctionDef { name: "or", func: or } }
inventory::submit! { FunctionDef { name: "xor", func: xor } }

// IF function
inventory::submit! { FunctionDef { name: "IF", func: if_func } }
inventory::submit! { FunctionDef { name: "if", func: if_func } }
