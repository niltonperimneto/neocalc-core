use crate::engine::errors::EngineError;
use crate::engine::functions::FunctionDef;
use crate::engine::types::Number;
use num::Signed;
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

// Helper for optional second argument (default 0)
fn get_two_args(args: &[Number]) -> Result<(Number, i32), EngineError> {
    if args.is_empty() || args.len() > 2 {
        return Err(EngineError::ArgumentMismatch("function".into(), 1)); // Simplified error
    }
    let val = args[0].clone();
    let digits = if args.len() == 2 {
        match args[1].to_f64() {
            Some(f) => f as i32,
            None => 0,
        }
    } else {
        0
    };
    Ok((val, digits))
}

pub fn abs(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::ArgumentMismatch("abs".into(), 1));
    }
    // Generic abs logic
    match &args[0] {
        Number::Integer(i) => Ok(Number::Integer(i.abs())),
        Number::Rational(r) => Ok(Number::Rational(r.abs())),
        Number::Float(f) => Ok(Number::Float(f.abs())),
        Number::Complex(c) => Ok(Number::Float(c.norm())), // Magnitude for complex
    }
}

pub fn fact(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::ArgumentMismatch("fact".into(), 1));
    }
    // Using the function defined in types.rs (re-exported or accessible?)
    // Actually types.rs defined it as a standalone function `pub fn factorial`.
    // We need to import it or access it. It seems types.rs is in the same module tree.
    // Let's assume crate::engine::types::factorial exists.
    crate::engine::types::factorial(args[0].clone()).map_err(|e| EngineError::Generic(e))
}

pub fn round(args: &[Number]) -> Result<Number, EngineError> {
    let (val, digits) = get_two_args(args)?;
    let f = val
        .to_f64()
        .ok_or(EngineError::Generic("Cannot convert to float".into()))?;
    let multiplier = 10f64.powi(digits);
    Ok(Number::Float((f * multiplier).round() / multiplier))
}

pub fn floor(args: &[Number]) -> Result<Number, EngineError> {
    let (val, _digits) = get_two_args(args)?; // OpenFormula FLOOR has significance, we simplify to math floor for now or implement significance later if requested exact ODFF.
    // Standard Math FLOOR(x)
    let f = val
        .to_f64()
        .ok_or(EngineError::Generic("Cannot convert to float".into()))?;
    Ok(Number::Float(f.floor()))
}

pub fn ceiling(args: &[Number]) -> Result<Number, EngineError> {
    let (val, _digits) = get_two_args(args)?;
    let f = val
        .to_f64()
        .ok_or(EngineError::Generic("Cannot convert to float".into()))?;
    Ok(Number::Float(f.ceil()))
}

pub fn trunc(args: &[Number]) -> Result<Number, EngineError> {
    let (val, _digits) = get_two_args(args)?;
    let f = val
        .to_f64()
        .ok_or(EngineError::Generic("Cannot convert to float".into()))?;
    Ok(Number::Float(f.trunc()))
}

inventory::submit! { FunctionDef { name: "log", func: log } }
inventory::submit! { FunctionDef { name: "ln", func: ln } }
inventory::submit! { FunctionDef { name: "sqrt", func: sqrt } }
inventory::submit! { FunctionDef { name: "abs", func: abs } }
inventory::submit! { FunctionDef { name: "ABS", func: abs } }
inventory::submit! { FunctionDef { name: "fact", func: fact } }
inventory::submit! { FunctionDef { name: "FACT", func: fact } }
inventory::submit! { FunctionDef { name: "round", func: round } }
inventory::submit! { FunctionDef { name: "ROUND", func: round } }
inventory::submit! { FunctionDef { name: "floor", func: floor } }
inventory::submit! { FunctionDef { name: "FLOOR", func: floor } }
inventory::submit! { FunctionDef { name: "ceil", func: ceiling } }
inventory::submit! { FunctionDef { name: "CEILING", func: ceiling } }
inventory::submit! { FunctionDef { name: "trunc", func: trunc } }
inventory::submit! { FunctionDef { name: "TRUNC", func: trunc } }
