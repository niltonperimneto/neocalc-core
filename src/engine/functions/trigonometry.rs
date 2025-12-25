use crate::engine::types::Number;
use crate::engine::errors::EngineError;
use crate::engine::functions::FunctionDef;
use num::complex::Complex64;

fn one_arg(args: &[Number], name: &str) -> Result<Complex64, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::ArgumentMismatch(name.into(), 1));
    }
    Ok(args[0].to_complex())
}

pub fn sin(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "sin")?.sin()))
}

pub fn cos(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "cos")?.cos()))
}

pub fn tan(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "tan")?.tan()))
}

pub fn asin(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "asin")?.asin()))
}

pub fn acos(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "acos")?.acos()))
}

pub fn atan(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "atan")?.atan()))
}

inventory::submit! { FunctionDef { name: "sin", func: sin } }
inventory::submit! { FunctionDef { name: "cos", func: cos } }
inventory::submit! { FunctionDef { name: "tan", func: tan } }
inventory::submit! { FunctionDef { name: "asin", func: asin } }
inventory::submit! { FunctionDef { name: "acos", func: acos } }
inventory::submit! { FunctionDef { name: "atan", func: atan } }
// Aliases
inventory::submit! { FunctionDef { name: "cosin", func: acos } }
