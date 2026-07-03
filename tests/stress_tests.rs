use neocalc_core::{
    evaluate,
    Context,
    Number,
};
use std::time::Instant;
use num_bigint::BigInt;

#[test]
fn stress_test_parser_depth() {
    let mut context = Context::new();
    // Generate "1 + 1 + ... + 1" (1000 times)
    // Left-associative: ((1+1)+1)+1...
    // Depth of AST ~ 1000.
    // Parser recursion depth ~ 1000.
    let n = 2000;
    let expr: String = vec!["1"; n].join(" + ");
    
    let start = Instant::now();
    let res = evaluate(&expr, &mut context);
    let duration = start.elapsed();
    
    match res {
        Ok(Number::Integer(i)) => {
            assert_eq!(i, BigInt::from(n), "Sum should be equal to count");
            println!("Parser Depth Test (N={}): {:?} - Parsed and Evaluated", n, duration);
        }
        Err(e) => {
            panic!("Parser failed at depth {}: {:?}", n, e);
        }
        _ => panic!("Expected Integer"),
    }
}

#[test]
fn stress_test_context_cloning_overhead() {
    let mut context = Context::new();
    
    // 1. Populate context with MANY variables
    let var_count = 10_000;
    println!("Populating {} variables...", var_count);
    for i in 0..var_count {
        evaluate(&format!("v{} = {}", i, i), &mut context).unwrap();
    }
    
    // 2. Define a simple function
    evaluate("f(x) = x + 1", &mut context).unwrap();
    
    // 3. Call it repeatedly
    let call_count = 1000;
    let start = Instant::now();
    for _ in 0..call_count {
        evaluate("f(1)", &mut context).unwrap();
    }
    let duration = start.elapsed();
    
    println!("Context Overhead Test (Vars={}, Calls={}): {:?}", var_count, call_count, duration);
    println!("Average time per call: {:?}", duration / call_count as u32);
    
    // If it takes > 1ms per call, it's slow. 
    // cloning 10k items 1000 times = 10M copies.
}

#[test]
fn stress_test_recursion_fib() {
    let mut context = Context::new();

    // Naive fibonacci via lazy `if` (no comparison operators, so the base
    // cases test `n-1` and `n-2` for zero): fib(1) = fib(2) = 1.
    evaluate(
        "fib(n) = if(n - 1, if(n - 2, fib(n - 1) + fib(n - 2), 1), 1)",
        &mut context,
    )
    .unwrap();

    let start = Instant::now();
    let res = evaluate("fib(20)", &mut context).unwrap();
    let duration = start.elapsed();

    assert_eq!(res, Number::Integer(BigInt::from(6765)));
    println!("Recursion Test fib(20): {:?}", duration);
}
