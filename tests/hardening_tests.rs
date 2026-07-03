use neocalc_core::{evaluate, Context, EngineError, Number};
use num_bigint::BigInt;

// --- Parser robustness -------------------------------------------------------

#[test]
fn test_invalid_trailing_operator() {
    // The motivating example: "12+=" must be rejected gracefully, not panic.
    let mut context = Context::new();
    let res = evaluate("12+=", &mut context);
    match res {
        Err(EngineError::ParserError(msg)) => {
            assert!(msg.contains("position"), "error should carry a position: {}", msg);
        }
        _ => panic!("Expected ParserError for '12+=', got {:?}", res),
    }
}

#[test]
fn test_various_malformed_inputs() {
    let cases = ["1 +", "(1", "1 2 3 +", "", "+", ")", "@"];
    for case in cases {
        let mut context = Context::new();
        let res = evaluate(case, &mut context);
        assert!(
            matches!(res, Err(EngineError::ParserError(_))),
            "Expected ParserError for {:?}, got {:?}",
            case,
            res
        );
    }
}

#[test]
fn test_deeply_nested_parens_no_overflow() {
    // Without a depth cap this would overflow the stack; it must error instead.
    let mut context = Context::new();
    let expr = format!("{}1{}", "(".repeat(100_000), ")".repeat(100_000));
    let res = evaluate(&expr, &mut context);
    assert!(
        matches!(res, Err(EngineError::ParserError(_))),
        "Expected ParserError for deep nesting, got {:?}",
        res
    );
}

#[test]
fn test_deep_right_associative_power_no_overflow() {
    // 2^2^2^...^2 nests to the right; the parser depth cap must catch it.
    let mut context = Context::new();
    let expr = vec!["2"; 100_000].join("^");
    let res = evaluate(&expr, &mut context);
    assert!(
        matches!(res, Err(EngineError::ParserError(_))),
        "Expected ParserError for deep power tower, got {:?}",
        res
    );
}

// --- Arithmetic safety -------------------------------------------------------

#[test]
fn test_modulo_by_zero_does_not_panic() {
    let mut context = Context::new();
    let res = evaluate("5 % 0", &mut context).unwrap();
    match res {
        Number::Float(f) => assert!(f.is_nan(), "5 % 0 should be NaN, got {}", f),
        other => panic!("Expected NaN float for 5 % 0, got {:?}", other),
    }
}

#[test]
fn test_division_by_zero_still_infinity() {
    // Division behavior is intentionally preserved (IEEE infinity).
    let mut context = Context::new();
    let res = evaluate("10 / 0", &mut context).unwrap();
    match res {
        Number::Float(f) => assert!(f.is_infinite(), "10 / 0 should be infinite, got {}", f),
        other => panic!("Expected infinite float for 10 / 0, got {:?}", other),
    }
}

// --- Resource limits ---------------------------------------------------------

#[test]
fn test_huge_power_is_rejected() {
    let mut context = Context::new();
    let res = evaluate("9^100000000", &mut context);
    assert!(
        matches!(res, Err(EngineError::ResourceLimit(_))),
        "Expected ResourceLimit for huge power, got {:?}",
        res
    );
}

#[test]
fn test_huge_factorial_is_rejected() {
    let mut context = Context::new();
    let res = evaluate("1000000!", &mut context);
    assert!(
        matches!(res, Err(EngineError::ResourceLimit(_))),
        "Expected ResourceLimit for huge factorial, got {:?}",
        res
    );
}

#[test]
fn test_constant_base_powers_are_not_rejected() {
    // Bases of -1, 0, 1 never grow, so large exponents must still evaluate.
    let mut context = Context::new();
    assert_eq!(
        evaluate("1^100000000", &mut context).unwrap(),
        Number::Integer(BigInt::from(1))
    );
    let mut context = Context::new();
    assert_eq!(
        evaluate("0^100000000", &mut context).unwrap(),
        Number::Integer(BigInt::from(0))
    );
    let mut context = Context::new();
    assert_eq!(
        evaluate("(-1)^100000000", &mut context).unwrap(),
        Number::Integer(BigInt::from(1))
    );
}

#[test]
fn test_non_real_rounding_is_type_error() {
    // A complex argument to round/floor is a type problem, not a math domain error.
    let mut context = Context::new();
    let res = evaluate("floor(sqrt(-1))", &mut context);
    assert!(
        matches!(res, Err(EngineError::TypeMismatch { .. })),
        "Expected TypeMismatch for floor of a complex number, got {:?}",
        res
    );
}

#[test]
fn test_reasonable_power_and_factorial_still_work() {
    let mut context = Context::new();
    assert!(evaluate("2^100", &mut context).is_ok());
    let mut context = Context::new();
    assert!(evaluate("50!", &mut context).is_ok());
}

#[test]
fn test_runaway_recursion_is_bounded() {
    let mut context = Context::new();
    evaluate("f(x) = f(x)", &mut context).unwrap();
    let res = evaluate("f(1)", &mut context);
    assert!(
        matches!(res, Err(EngineError::ResourceLimit(_))),
        "Expected ResourceLimit for infinite recursion, got {:?}",
        res
    );
}

// --- Result simplification ---------------------------------------------------

#[test]
fn test_sqrt_of_perfect_square_is_real() {
    let mut context = Context::new();
    let res = evaluate("sqrt(4)", &mut context).unwrap();
    // Should be simplified to a real number, not a Complex value.
    match res {
        Number::Float(f) => assert!((f - 2.0).abs() < 1e-9, "sqrt(4) should be 2, got {}", f),
        Number::Integer(i) => assert_eq!(i, BigInt::from(2)),
        other => panic!("sqrt(4) should be real, got {:?}", other),
    }
}

#[test]
fn test_whole_rational_simplifies_to_integer() {
    let mut context = Context::new();
    let res = evaluate("10 / 2", &mut context).unwrap();
    match res {
        Number::Integer(i) => assert_eq!(i, BigInt::from(5)),
        other => panic!("10 / 2 should simplify to Integer(5), got {:?}", other),
    }
}

#[test]
fn test_bitwise_accepts_whole_rational() {
    // band requires integers; a rational-valued integer must be accepted.
    let mut context = Context::new();
    let res = evaluate("band(10 / 2, 1)", &mut context).unwrap();
    match res {
        Number::Integer(i) => assert_eq!(i, BigInt::from(1)),
        other => panic!("band(10/2, 1) should be Integer(1), got {:?}", other),
    }
}

// --- Logic / control flow ----------------------------------------------------

#[test]
fn test_if_is_lazy() {
    // The untaken branch (1/0 -> infinity, but here a resource-heavy expr) must
    // not affect the result; use a branch that would error if evaluated.
    let mut context = Context::new();
    let res = evaluate("if(1, 2, 1000000!)", &mut context).unwrap();
    match res {
        Number::Integer(i) => assert_eq!(i, BigInt::from(2)),
        other => panic!("if(1, 2, ...) should be 2, got {:?}", other),
    }
}

#[test]
fn test_if_lazy_enables_recursion() {
    // Countdown recursion terminates only if IF short-circuits the base case.
    let mut context = Context::new();
    evaluate("countdown(n) = if(n, countdown(n - 1), 0)", &mut context).unwrap();
    let res = evaluate("countdown(10)", &mut context).unwrap();
    match res {
        Number::Integer(i) => assert_eq!(i, BigInt::from(0)),
        other => panic!("countdown(10) should be 0, got {:?}", other),
    }
}

#[test]
fn test_nan_is_falsy() {
    let mut context = Context::new();
    let res = evaluate("if(0 / 0, 1, 2)", &mut context).unwrap();
    match res {
        Number::Integer(i) => assert_eq!(i, BigInt::from(2)),
        other => panic!("if(0/0, 1, 2) should be 2 (NaN falsy), got {:?}", other),
    }
}

// --- Function-name fixes -----------------------------------------------------

#[test]
fn test_cosin_alias_is_cosine() {
    let mut context = Context::new();
    let res = evaluate("cosin(0)", &mut context).unwrap();
    match res {
        Number::Float(f) => assert!((f - 1.0).abs() < 1e-9, "cosin(0) should be 1, got {}", f),
        Number::Integer(i) => assert_eq!(i, BigInt::from(1)),
        other => panic!("cosin(0) should be 1, got {:?}", other),
    }
}

// --- Human-readable error messages ------------------------------------------

fn error_message(expr: &str) -> String {
    let mut context = Context::new();
    match evaluate(expr, &mut context) {
        Ok(n) => panic!("expected an error for {:?}, got {:?}", expr, n),
        Err(e) => e.to_string(),
    }
}

#[test]
fn test_error_messages_are_human_readable() {
    // Syntax errors point at the offending position.
    let msg = error_message("12+=");
    assert!(
        msg.starts_with("Syntax error:") && msg.contains("position 3"),
        "got: {}",
        msg
    );

    // Argument-count errors say what was expected and what was received.
    let msg = error_message("abs(1, 2)");
    assert!(
        msg.contains("expects 1 argument") && msg.contains("got 2"),
        "got: {}",
        msg
    );

    // Variable-arity functions describe the accepted range.
    let msg = error_message("fv(1)");
    assert!(msg.contains("3 to 5 arguments"), "got: {}", msg);

    // Unknown functions and variables name the offending identifier.
    assert!(error_message("foo(3)").contains("Unknown function 'foo'"));
    assert!(error_message("y").contains("Undefined variable 'y'"));

    // Type errors report the actual kind received (not a reversed template).
    let msg = error_message("band(1.5, 1)");
    assert!(
        msg.contains("expected a whole number") && msg.contains("got a real number"),
        "got: {}",
        msg
    );

    // Resource limits explain the cap.
    assert!(error_message("1000000!").contains("Computation too large"));
}

#[test]
fn test_round_absurd_digits_is_finite() {
    let mut context = Context::new();
    let res = evaluate("round(5, 1000000000)", &mut context).unwrap();
    match res {
        Number::Float(f) => assert!(f.is_finite(), "round with absurd digits should be finite"),
        Number::Integer(_) => {}
        other => panic!("round(5, 1e9) should be finite, got {:?}", other),
    }
}

// --- Bit-shift and financial-argument bounds ---------------------------------

#[test]
fn test_huge_left_shift_is_rejected() {
    // A left shift by an enormous count would allocate the result's full bit
    // width (~125 GB here); it must be refused, not attempted.
    let mut context = Context::new();
    let res = evaluate("lsh(1, 999999999999)", &mut context);
    assert!(
        matches!(res, Err(EngineError::ResourceLimit(_))),
        "Expected ResourceLimit for huge lsh, got {:?}",
        res
    );
}

#[test]
fn test_reasonable_left_shift_still_works() {
    let mut context = Context::new();
    let res = evaluate("lsh(1, 8)", &mut context).unwrap();
    assert_eq!(res, Number::Integer(BigInt::from(256)));
}

#[test]
fn test_irr_requires_arguments() {
    // With no cash flows, irr used to return a meaningless 0.1 guess.
    let mut context = Context::new();
    let res = evaluate("irr()", &mut context);
    assert!(
        matches!(res, Err(EngineError::ArgumentMismatch { .. })),
        "Expected ArgumentMismatch for irr(), got {:?}",
        res
    );
}
