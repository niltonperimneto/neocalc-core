use neocalc_core::{Context, Number, evaluate};
use num_bigint::BigInt;

#[test]
fn test_global_scope_update() {
    let mut context = Context::new();

    // Initialize x in global scope
    evaluate("x = 10", &mut context).unwrap();

    // Function that modifies x
    // With dynamic scoping and "update-or-define", this should update global x
    // because x exists in the scope chain.
    evaluate("f() = x = 20", &mut context).unwrap();

    // Call f()
    evaluate("f()", &mut context).unwrap();

    // Check global x
    let res = evaluate("x", &mut context).unwrap();
    if let Number::Integer(i) = res {
        assert_eq!(i, BigInt::from(20), "Global x should be updated to 20");
    } else {
        panic!("Expected Integer");
    }
}

#[test]
fn test_local_shadowing() {
    let mut context = Context::new();

    // Global x = 10
    evaluate("x = 10", &mut context).unwrap();

    // Function taking x as parameter - this CREATES a new local scope with x
    evaluate("g(x) = x = 20", &mut context).unwrap();

    // Call g(5)
    evaluate("g(5)", &mut context).unwrap();

    // Global x should REMAIN 10, because g's x was a local parameter
    let res = evaluate("x", &mut context).unwrap();
    if let Number::Integer(i) = res {
        assert_eq!(i, BigInt::from(10), "Global x should remain 10");
    } else {
        panic!("Expected Integer");
    }
}

#[test]
fn test_nested_function_update() {
    let mut context = Context::new();

    evaluate("global_var = 100", &mut context).unwrap();

    // Outer function
    evaluate("outer() = inner()", &mut context).unwrap();

    // Inner function updates global_var
    evaluate("inner() = global_var = 200", &mut context).unwrap();

    evaluate("outer()", &mut context).unwrap();

    let res = evaluate("global_var", &mut context).unwrap();
    if let Number::Integer(i) = res {
        assert_eq!(
            i,
            BigInt::from(200),
            "Global var should be updated via nested call"
        );
    } else {
        panic!("Expected Integer");
    }
}

#[test]
fn test_local_definition() {
    let mut context = Context::new();

    // Function defines a NEW variable y (not existing globally)
    // It should be defined in the local scope of h(), and eventually popped?
    // Wait. `h() = y = 5`.
    // execute `y=5`. `y` not found. Define in TOP scope (which is local to h).
    // Scope pops. `y` is lost.
    evaluate("h() = y = 5", &mut context).unwrap();

    evaluate("h()", &mut context).unwrap();

    // y should NOT exist globally
    let res = evaluate("y", &mut context);
    match res {
        Err(neocalc_core::EngineError::UndefinedVariable(_)) => (),
        _ => panic!("y should be undefined globally"),
    }
}
