use neocalc_core::{
    Number,
    utils::{format_number, format_number_decimal},
};
use num_bigint::BigInt;
use num_rational::BigRational;

#[test]
fn test_format_decimal() {
    // 1/2
    let n = Number::Rational(BigRational::new(BigInt::from(1), BigInt::from(2)));
    assert_eq!(format_number(n.clone()), "1/2");
    assert_eq!(format_number_decimal(n), "0.5");

    // 1/3
    let n = Number::Rational(BigRational::new(BigInt::from(1), BigInt::from(3)));
    assert_eq!(format_number(n.clone()), "1/3");
    // 1/3 is approx 0.3333333333333333
    // format_float uses to_string(), which typically has 15-17 digits for f64
    let s = format_number_decimal(n);
    assert!(s.starts_with("0.333333"));

    // Integer 5
    let n = Number::Integer(BigInt::from(5));
    assert_eq!(format_number(n.clone()), "5");
    assert_eq!(format_number_decimal(n), "5");

    // Integer 5/1 as Rational
    let n = Number::Rational(BigRational::new(BigInt::from(5), BigInt::from(1)));
    assert_eq!(format_number(n.clone()), "5");
    assert_eq!(format_number_decimal(n), "5");
}
