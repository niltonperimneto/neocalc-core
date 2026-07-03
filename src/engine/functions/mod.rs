pub mod bitwise;
pub mod complex_ops;
pub mod core_funcs;
pub mod financial;
pub mod hyperbolic;
pub mod logic;
pub mod statistics;
pub mod trigonometry;

use crate::engine::errors::EngineError;
use crate::engine::types::Number;
use num::complex::Complex64;

use std::collections::HashMap;
use std::sync::OnceLock;

/// Signature shared by every built-in function.
pub type BuiltinFn = fn(&[Number]) -> Result<Number, EngineError>;

pub struct FunctionDef {
    pub name: &'static str,
    pub func: BuiltinFn,
}

inventory::collect!(FunctionDef);

/// Validate a single-argument call and return the argument as a complex value.
/// Shared by the many one-argument analytic functions (sin, ln, sqrt, ...).
pub(crate) fn one_arg(args: &[Number], name: &str) -> Result<Complex64, EngineError> {
    if args.len() != 1 {
        return Err(EngineError::arity(name, 1, args.len()));
    }
    Ok(args[0].to_complex())
}

/// Reject empty argument lists for variadic functions (mean, median, ...).
pub(crate) fn require_nonempty(args: &[Number], name: &str) -> Result<(), EngineError> {
    if args.is_empty() {
        return Err(EngineError::ArgumentMismatch {
            name: name.into(),
            expected: "at least 1 argument".into(),
            got: 0,
        });
    }
    Ok(())
}

static FUNCTION_REGISTRY: OnceLock<HashMap<&'static str, BuiltinFn>> = OnceLock::new();

fn get_registry() -> &'static HashMap<&'static str, BuiltinFn> {
    FUNCTION_REGISTRY.get_or_init(|| {
        let mut m = HashMap::new();
        for func_def in inventory::iter::<FunctionDef> {
            let previous = m.insert(func_def.name, func_def.func);
            // A name registered twice means one implementation silently wins;
            // catch that in development instead of shipping the ambiguity.
            debug_assert!(
                previous.is_none(),
                "function '{}' registered more than once",
                func_def.name
            );
        }
        m
    })
}

pub fn apply(name: &str, args: Vec<Number>) -> Result<Number, EngineError> {
    let registry = get_registry();
    match registry.get(name) {
        Some(func) => func(&args),
        None => Err(EngineError::UnknownFunction(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::FunctionDef;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_function_registrations() {
        let mut seen = HashSet::new();
        for def in inventory::iter::<FunctionDef> {
            assert!(
                seen.insert(def.name),
                "function '{}' registered more than once",
                def.name
            );
        }
    }
}
