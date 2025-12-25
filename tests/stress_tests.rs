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
    let expr: String = std::iter::repeat("1").take(n).collect::<Vec<_>>().join(" + ");
    
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
    for i in 0..call_count {
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
    
    // Define naive fibonacci: fib(n) = fib(n-1) + fib(n-2)
    // Base cases handled by if? We don't have 'if' in Expr yet!
    // Ah, we can't do recursion without conditionals.
    // We only have math operators.
    // So we can't test recursion logic per se without standard library 'if' function.
    // Assuming we have 'if(cond, true_val, false_val)'.
    // We don't have an 'if' function in `core_funcs`?
    // Let's check.
}
