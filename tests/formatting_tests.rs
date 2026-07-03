use neocalc_core::{Number, utils::format_number};
use num_bigint::BigInt;
use num_rational::BigRational;

#[test]
fn test_format_decimal() {
    // 1/2
    let n = Number::Rational(BigRational::new(BigInt::from(1), BigInt::from(2)));
    assert_eq!(format_number(n.clone(), false), "1/2");
    assert_eq!(format_number(n, true), "0.5");

    // 1/3
    let n = Number::Rational(BigRational::new(BigInt::from(1), BigInt::from(3)));
    assert_eq!(format_number(n.clone(), false), "1/3");
    // 1/3 is approx 0.3333333333333333
    // format_float uses to_string(), which typically has 15-17 digits for f64
    let s = format_number(n, true);
    assert!(s.starts_with("0.333333"));

    // Integer 5
    let n = Number::Integer(BigInt::from(5));
    assert_eq!(format_number(n.clone(), false), "5");
    assert_eq!(format_number(n, true), "5");

    // Integer 5/1 as Rational
    let n = Number::Rational(BigRational::new(BigInt::from(5), BigInt::from(1)));
    assert_eq!(format_number(n.clone(), false), "5");
    assert_eq!(format_number(n, true), "5");
}

#[test]
fn test_large_floats_do_not_saturate_to_i64() {
    // 1e300 has a zero fractional part, but must not be printed through the
    // i64 path (which would saturate to 9223372036854775807).
    let s = format_number(Number::Float(1e300), true);
    assert_ne!(s, i64::MAX.to_string());
    assert_eq!(s.parse::<f64>().unwrap(), 1e300);

    let s = format_number(Number::Float(-1e300), true);
    assert_ne!(s, i64::MIN.to_string());
    assert_eq!(s.parse::<f64>().unwrap(), -1e300);

    // Non-finite values fall through to the standard float formatting.
    assert_eq!(format_number(Number::Float(f64::INFINITY), true), "inf");
    assert!(format_number(Number::Float(f64::NAN), true) == "NaN");

    // Values that fit exactly keep the clean integer rendering.
    assert_eq!(format_number(Number::Float(42.0), true), "42");
}
