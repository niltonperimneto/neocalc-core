use crate::engine::types::Number;
use crate::engine::errors::EngineError;
use crate::engine::functions::{FunctionDef, one_arg};

pub fn sinh(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "sinh")?.sinh()))
}

pub fn cosh(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "cosh")?.cosh()))
}

pub fn tanh(args: &[Number]) -> Result<Number, EngineError> {
    Ok(Number::Complex(one_arg(args, "tanh")?.tanh()))
}

inventory::submit! { FunctionDef { name: "sinh", func: sinh } }
inventory::submit! { FunctionDef { name: "cosh", func: cosh } }
inventory::submit! { FunctionDef { name: "tanh", func: tanh } }
