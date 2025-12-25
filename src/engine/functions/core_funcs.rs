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

pub fn log(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "log")?.log(10.0)))
}

pub fn ln(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "ln")?.ln()))
}

pub fn sqrt(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "sqrt")?.sqrt()))
}

inventory::submit! { FunctionDef { name: "log", func: log } }
inventory::submit! { FunctionDef { name: "ln", func: ln } }
inventory::submit! { FunctionDef { name: "sqrt", func: sqrt } }
