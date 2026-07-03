use crate::engine::errors::EngineError;
use crate::engine::functions::FunctionDef;
use crate::engine::types::Number;

pub fn conj(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::arity("conj", 1, args.len()));
    }
    match &args[0] {
        Number::Complex(c) => Ok(Number::Complex(c.conj())),
        n => Ok(n.clone()), // Real numbers are their own conjugate
    }
}

pub fn re(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::arity("re", 1, args.len()));
    }
    match &args[0] {
        Number::Complex(c) => Ok(Number::Float(c.re)), // Complex parts are floats
        n => Ok(n.clone()),                            // Real part of real is self
    }
}

pub fn im(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::arity("im", 1, args.len()));
    }
    match &args[0] {
        Number::Complex(c) => Ok(Number::Float(c.im)),
        Number::Integer(_) | Number::Rational(_) => {
            Ok(Number::Integer(num_bigint::BigInt::from(0)))
        }
        Number::Float(_) => Ok(Number::Float(0.0)),
    }
}

inventory::submit! { FunctionDef { name: "conj", func: conj } }
inventory::submit! { FunctionDef { name: "re", func: re } }
inventory::submit! { FunctionDef { name: "im", func: im } }
inventory::submit! { FunctionDef { name: "Im", func: im } } // Alias
