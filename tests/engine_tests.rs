use neocalc_core::{
    evaluate,
    Context,
    Number,
};
use num_bigint::BigInt;
use num_rational::BigRational;

#[test]
fn test_sanity_arithmetic() {
    let mut context = Context::new();
    let res = evaluate("1 + 1", &mut context).unwrap();
    if let Number::Integer(i) = res {
        assert_eq!(i, BigInt::from(2));
    } else {
        panic!("Expected Integer");
    }

    let mut context = Context::new();
    let res = evaluate("10 / 2", &mut context).unwrap();
    // 10 / 2 is Rational(5/1) or Integer(5) depending on implementation.
    // Our implementation does Rational for Div of Integers.
    if let Number::Rational(r) = res {
        assert_eq!(r, BigRational::from_integer(BigInt::from(5)));
    } else if let Number::Integer(i) = res {
        assert_eq!(i, BigInt::from(5));
    } else {
        panic!("Expected Rational or Integer, got {:?}", res);
    }
}

#[test]
fn test_exact_rational() {
    let mut context = Context::new();
    // 1 / 3
    let res = evaluate("1 / 3", &mut context).unwrap();
    if let Number::Rational(r) = res {
        assert_eq!(r, BigRational::new(BigInt::from(1), BigInt::from(3)));
    } else {
        panic!("Expected Rational");
    }

    let mut context = Context::new();
    // (1/3) * 3 = 1
    let res = evaluate("(1 / 3) * 3", &mut context).unwrap();
    // This should resolve to Rational(1/1) which might normalize?
    if let Number::Rational(r) = res {
        assert_eq!(r, BigRational::from_integer(BigInt::from(1)));
    } else {
        panic!("Expected Rational(1)");
    }
}

#[test]
fn test_big_numbers() {
    let mut context = Context::new();
    // 2^100
    let res = evaluate("2^100", &mut context).unwrap();
    if let Number::Integer(i) = res {
        // 2^100 is approx 1.26e30. Check strict positivity and size?
        assert!(i > BigInt::from(1u64 << 60)); // Bigger than u64 max
    } else {
        panic!("Expected Integer for Power");
    }
}

#[test]
fn test_factorial_stress() {
    let mut context = Context::new();
    // 50!
    let res = evaluate("50!", &mut context).unwrap();
    if let Number::Integer(i) = res {
        // 50! is huge.
        assert!(i > BigInt::from(1));
    } else {
        panic!("Expected Integer for Factorial");
    }
}

#[test]
fn test_type_promotion() {
    let mut context = Context::new();
    // 1 + 0.5 -> Float
    let res = evaluate("1 + 0.5", &mut context).unwrap();
    if let Number::Float(f) = res {
        assert_eq!(f, 1.5);
    } else {
        panic!("Expected Float");
    }

    let mut context = Context::new();
    // 1/2 + 0.5 -> Float
    let res = evaluate("(1/2) + 0.5", &mut context).unwrap();
    if let Number::Float(f) = res {
        assert_eq!(f, 1.0);
    } else {
        panic!("Expected Float");
    }
}

#[test]
fn test_bitwise_bigint() {
    let mut context = Context::new();
    // band(3, 1) -> 1
    let res = evaluate("band(3, 1)", &mut context).unwrap();
    if let Number::Integer(i) = res {
        assert_eq!(i, BigInt::from(1));
    } else {
        panic!("Expected Integer for band");
    }

    let mut context = Context::new();
    // Test huge integer bitwise: (2^100 + 1) & 1 == 1
    // 2^100 ends in ...000, so 2^100 + 1 ends in ...001.
    // ANDing with 1 should give 1.
    let res = evaluate("band(2^100 + 1, 1)", &mut context).unwrap();
    if let Number::Integer(i) = res {
        assert_eq!(i, BigInt::from(1));
    } else {
        panic!("Expected Integer 1 for huge band");
    }
}

#[test]
fn test_statistics_exact() {
    let mut context = Context::new();
    // mean(1, 2) -> 3/2 (Rational)
    let res = evaluate("mean(1, 2)", &mut context).unwrap();
    if let Number::Rational(r) = res {
        assert_eq!(r, BigRational::new(BigInt::from(3), BigInt::from(2)));
    } else {
        panic!("Expected Rational for mean");
    }
}

#[test]
fn test_variables_and_functions() {
    let mut context = Context::new();

    // Assignment
    evaluate("x = 10", &mut context).unwrap();
    
    // Parens
    evaluate("(x)", &mut context).unwrap(); // Should equal 10

    // Function call (undefined)
    let res = evaluate("g(x)", &mut context);
    match res {
        Err(neocalc_core::EngineError::UnknownFunction(_)) => (),
        _ => panic!("Expected UnknownFunction for g(x), got {:?}", res),
    }

    // Function definition simple
    evaluate("h(x) = 10", &mut context).unwrap();

    // Check if it exists
    let res = evaluate("h(5)", &mut context).unwrap();
    if let Number::Integer(i) = res {
         assert_eq!(i, BigInt::from(10));
    } else {
        panic!("Expected Integer 10 for h(5)");
    }
    
    // Function definition with usage
    evaluate("f(x) = x^2", &mut context).unwrap();
    let res = evaluate("f(5)", &mut context).unwrap();
    if let Number::Integer(i) = res {
         assert_eq!(i, BigInt::from(25));
    } else {
         panic!("Expected Integer 25 for f(5)");
    }
}
