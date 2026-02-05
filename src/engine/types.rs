use crate::engine::errors::EngineError;
use num::complex::Complex64;
use num::traits::Pow;
use num::{One, ToPrimitive, Zero};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Number {
    Integer(BigInt),
    Rational(BigRational),
    Float(f64),
    #[serde(with = "complex_serde")]
    Complex(Complex64),
}

/// Custom serde for Complex64 since it doesn't implement Serialize/Deserialize
mod complex_serde {
    use num::complex::Complex64;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct ComplexRepr {
        re: f64,
        im: f64,
    }

    pub fn serialize<S>(c: &Complex64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ComplexRepr { re: c.re, im: c.im }.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Complex64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = ComplexRepr::deserialize(deserializer)?;
        Ok(Complex64::new(repr.re, repr.im))
    }
}

impl Number {
    pub fn to_complex(&self) -> Complex64 {
        match self {
            Number::Integer(i) => Complex64::new(i.to_f64().unwrap_or(f64::INFINITY), 0.0),
            Number::Rational(r) => Complex64::new(r.to_f64().unwrap_or(f64::NAN), 0.0),
            Number::Float(f) => Complex64::new(*f, 0.0),
            Number::Complex(c) => *c,
        }
    }

    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Number::Integer(i) => i.to_f64(),
            Number::Rational(r) => r.to_f64(),
            Number::Float(f) => Some(*f),
            Number::Complex(c) => {
                if c.im == 0.0 {
                    Some(c.re)
                } else {
                    None
                }
            }
        }
    }
    // Number struct definition
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match promote(self.clone(), other.clone()) {
            (Number::Integer(l), Number::Integer(r)) => Some(l.cmp(&r)),
            (Number::Rational(l), Number::Rational(r)) => Some(l.cmp(&r)),
            (Number::Float(l), Number::Float(r)) => l.partial_cmp(&r),
            (Number::Complex(_), _) | (_, Number::Complex(_)) => None,
            _ => None, // Should be unreachable given promote
        }
    }
}

// Helper to promote types
// Rank: Integer (0) -> Rational (1) -> Float (2) -> Complex (3)
fn promote(lhs: Number, rhs: Number) -> (Number, Number) {
    match (lhs, rhs) {
        (Number::Integer(l), Number::Integer(r)) => (Number::Integer(l), Number::Integer(r)),

        // Integer vs Rational -> Rational
        (Number::Integer(l), Number::Rational(r)) => (
            Number::Rational(BigRational::from_integer(l)),
            Number::Rational(r),
        ),
        (Number::Rational(l), Number::Integer(r)) => (
            Number::Rational(l),
            Number::Rational(BigRational::from_integer(r)),
        ),
        (Number::Rational(l), Number::Rational(r)) => (Number::Rational(l), Number::Rational(r)),

        // Anything vs Float -> Float (Note: Rational -> Float loses precision)
        (Number::Float(l), r) => (
            Number::Float(l),
            Number::Float(r.to_f64().unwrap_or(f64::NAN)),
        ),
        (l, Number::Float(r)) => (
            Number::Float(l.to_f64().unwrap_or(f64::NAN)),
            Number::Float(r),
        ),

        // Anything vs Complex -> Complex
        (Number::Complex(l), r) => (Number::Complex(l), Number::Complex(r.to_complex())),
        (l, Number::Complex(r)) => (Number::Complex(l.to_complex()), Number::Complex(r)),
    }
}

// Macro to implement binary operations +, -, *
macro_rules! impl_binary_op {
    ($trait:ident, $method:ident) => {
        impl $trait for Number {
            type Output = Number;
            fn $method(self, rhs: Self) -> Self::Output {
                match promote(self, rhs) {
                    (Number::Integer(l), Number::Integer(r)) => Number::Integer(l.$method(r)),
                    (Number::Rational(l), Number::Rational(r)) => Number::Rational(l.$method(r)),
                    (Number::Float(l), Number::Float(r)) => Number::Float(l.$method(r)),
                    (Number::Complex(l), Number::Complex(r)) => Number::Complex(l.$method(r)),
                    _ => unreachable!("Promote should have handled all type combinations"),
                }
            }
        }
    };
}

impl_binary_op!(Add, add);
impl_binary_op!(Sub, sub);
impl_binary_op!(Mul, mul);

impl Div for Number {
    type Output = Number;
    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            // Special Case: Integer / Integer = Rational to preserve precision
            (Number::Integer(l), Number::Integer(r)) => {
                if r.is_zero() {
                    // Division by zero; promote to float to produce INF
                    return Number::Float(l.to_f64().unwrap_or(f64::NAN) / 0.0);
                }
                Number::Rational(BigRational::new(l, r))
            }

            (l, r) => match promote(l, r) {
                (Number::Rational(l), Number::Rational(r)) => {
                    if r.is_zero() {
                        // Let float division handle infinity.
                        return Number::Float(l.to_f64().unwrap_or(f64::NAN) / 0.0);
                    }
                    Number::Rational(l / r)
                }
                (Number::Float(l), Number::Float(r)) => Number::Float(l / r),
                (Number::Complex(l), Number::Complex(r)) => Number::Complex(l / r),
                _ => unreachable!(),
            },
        }
    }
}

impl Neg for Number {
    type Output = Number;
    fn neg(self) -> Self::Output {
        match self {
            Number::Integer(i) => Number::Integer(-i),
            Number::Rational(r) => Number::Rational(-r),
            Number::Float(f) => Number::Float(-f),
            Number::Complex(c) => Number::Complex(-c),
        }
    }
}

impl Rem for Number {
    type Output = Number;
    fn rem(self, rhs: Self) -> Self::Output {
        match promote(self, rhs) {
            (Number::Integer(l), Number::Integer(r)) => Number::Integer(l % r),
            (Number::Rational(l), Number::Rational(r)) => Number::Rational(l % r),
            (Number::Float(l), Number::Float(r)) => Number::Float(l % r),
            // The remainder operator is not well-defined for complex numbers.
            // Returning NaN is a safe way to signal an invalid operation.
            (Number::Complex(_), Number::Complex(_)) => Number::Float(f64::NAN),
            _ => unreachable!(),
        }
    }
}

pub fn pow(base: Number, exp: Number) -> Number {
    match (base, exp) {
        // Power of integer by integer
        (Number::Integer(b), Number::Integer(e)) => {
            // Positive exponent
            if e >= BigInt::zero() {
                if let Some(e_u32) = e.to_u32() {
                    return Number::Integer(b.pow(e_u32));
                }
            }
            // Negative exponent
            else {
                if let Some(e_u32) = (-&e).to_u32() {
                    let den = b.pow(e_u32);
                    if den.is_zero() {
                        return Number::Float(f64::INFINITY);
                    } // e.g. 0^-2 -> inf
                    return Number::Rational(BigRational::new(BigInt::one(), den));
                }
            }
            // Exponent is too large for u32, fall back to float calculation
            let b_f64 = b.to_f64().unwrap_or(f64::NAN);
            let e_f64 = e.to_f64().unwrap_or(f64::NAN);
            Number::Float(b_f64.powf(e_f64))
        }
        // Fallback to complex powers for all other cases
        (b, e) => Number::Complex(b.to_complex().powc(e.to_complex())),
    }
}

pub fn factorial(n: Number) -> Result<Number, EngineError> {
    match n {
        Number::Integer(i) => {
            if i < BigInt::zero() {
                return Err(EngineError::DomainError(
                    "Factorial of negative integer".into(),
                ));
            }
            // Warning: Huge loop for big integers.
            // Simplified loop:
            let mut acc = BigInt::one();
            let mut k = BigInt::one();
            while k <= i {
                acc = acc * &k;
                k = k + 1;
                // Safety brake? No, user asked for "Infinite" calculator.
            }
            Ok(Number::Integer(acc))
        }
        _ => Err(EngineError::DomainError(
            "Factorial only implemented for Integers currently".into(),
        )),
    }
}
