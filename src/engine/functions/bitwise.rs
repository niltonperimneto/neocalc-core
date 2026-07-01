use crate::engine::types::Number;
use crate::engine::errors::EngineError;
use crate::engine::functions::FunctionDef;
use num::ToPrimitive;
use num_bigint::BigInt;

fn to_int(n: &Number) -> Result<BigInt, EngineError> {
    match n {
        Number::Integer(i) => Ok(i.clone()),
        other => Err(EngineError::TypeMismatch {
            expected: "a whole number (bitwise operations only work on integers)".into(),
            got: other.type_name().into(),
        }),
    }
}

fn apply_binary_op<F>(args: &[Number], name: &str, op: F) -> Result<Number, EngineError>
where
    F: Fn(BigInt, BigInt) -> Result<Number, EngineError>,
{
    if args.len() != 2 {
        return Err(EngineError::arity(name, 2, args.len()));
    }
    let a = to_int(&args[0])?;
    let b = to_int(&args[1])?;
    op(a, b)
}

pub fn band(args: &[Number]) -> Result<Number, EngineError> {
    apply_binary_op(args, "band", |a, b| Ok(Number::Integer(a & b)))
}

pub fn bor(args: &[Number]) -> Result<Number, EngineError> {
    apply_binary_op(args, "bor", |a, b| Ok(Number::Integer(a | b)))
}

pub fn bxor(args: &[Number]) -> Result<Number, EngineError> {
    apply_binary_op(args, "bxor", |a, b| Ok(Number::Integer(a ^ b)))
}

pub fn bnot(args: &[Number]) -> Result<Number, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::arity("bnot", 1, args.len()));
    }
    let a = to_int(&args[0])?;
    Ok(Number::Integer(!a))
}

pub fn lsh(args: &[Number]) -> Result<Number, EngineError> {
    apply_binary_op(args, "lsh", |a, b| {
         if let Some(shift) = b.to_usize() {
             Ok(Number::Integer(a << shift))
         } else {
             Err(EngineError::DomainError("shift count must be a non-negative whole number".into()))
         }
    })
}

pub fn rsh(args: &[Number]) -> Result<Number, EngineError> {
    apply_binary_op(args, "rsh", |a, b| {
         if let Some(shift) = b.to_usize() {
             Ok(Number::Integer(a >> shift))
         } else {
             Err(EngineError::DomainError("shift count must be a non-negative whole number".into()))
         }
    })
}

pub fn rol(args: &[Number]) -> Result<Number, EngineError> {
    apply_binary_op(args, "rol", |a, b| {
        if let (Some(val), Some(rot)) = (a.to_i64(), b.to_u32()) {
            Ok(Number::Integer(BigInt::from(val.rotate_left(rot))))
        } else {
            Err(EngineError::DomainError("rotate needs a 64-bit integer value and a rotation count that fits in 32 bits".into()))
        }
    })
}

pub fn ror(args: &[Number]) -> Result<Number, EngineError> {
    apply_binary_op(args, "ror", |a, b| {
        if let (Some(val), Some(rot)) = (a.to_i64(), b.to_u32()) {
            Ok(Number::Integer(BigInt::from(val.rotate_right(rot))))
        } else {
            Err(EngineError::DomainError("rotate needs a 64-bit integer value and a rotation count that fits in 32 bits".into()))
        }
    })
}

inventory::submit! { FunctionDef { name: "band", func: band } }
inventory::submit! { FunctionDef { name: "bor", func: bor } }
inventory::submit! { FunctionDef { name: "bxor", func: bxor } }
inventory::submit! { FunctionDef { name: "bnot", func: bnot } }
inventory::submit! { FunctionDef { name: "lsh", func: lsh } }
inventory::submit! { FunctionDef { name: "rsh", func: rsh } }
inventory::submit! { FunctionDef { name: "rol", func: rol } }
inventory::submit! { FunctionDef { name: "ror", func: ror } }
