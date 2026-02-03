use crate::engine::errors::EngineError;
use crate::engine::functions::FunctionDef;
use crate::engine::types::Number;
use num::Signed;

pub fn conj(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::ArgumentMismatch("conj".into(), 1));
    }
    match &args[0] {
        Number::Complex(c) => Ok(Number::Complex(c.conj())),
        n => Ok(n.clone()), // Real numbers are their own conjugate
    }
}

pub fn re(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::ArgumentMismatch("re".into(), 1));
    }
    match &args[0] {
        Number::Complex(c) => Ok(Number::Float(c.re)), // Complex parts are floats
        n => Ok(n.clone()),                            // Real part of real is self
    }
}

pub fn im(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::ArgumentMismatch("im".into(), 1));
    }
    match &args[0] {
        Number::Complex(c) => Ok(Number::Float(c.im)),
        Number::Integer(_) | Number::Rational(_) => {
            Ok(Number::Integer(num_bigint::BigInt::from(0)))
        }
        Number::Float(_) => Ok(Number::Float(0.0)),
    }
}

pub fn abs(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::ArgumentMismatch("abs".into(), 1));
    }
    match &args[0] {
        Number::Integer(i) => Ok(Number::Integer(i.abs())),
        Number::Rational(r) => Ok(Number::Rational(r.abs())),
        Number::Float(f) => Ok(Number::Float(f.abs())),
        Number::Complex(c) => Ok(Number::Float(c.norm())),
    }
}

inventory::submit! { FunctionDef { name: "conj", func: conj } }
inventory::submit! { FunctionDef { name: "re", func: re } }
inventory::submit! { FunctionDef { name: "im", func: im } }
inventory::submit! { FunctionDef { name: "lm", func: im } } // Alias
inventory::submit! { FunctionDef { name: "abs", func: abs } }
