pub mod bitwise;
pub mod complex_ops;
pub mod core_funcs;
pub mod financial;
pub mod hyperbolic;
pub mod statistics;
pub mod trigonometry;

use crate::engine::types::Number;
use crate::engine::errors::EngineError;

use std::collections::HashMap;
use std::sync::OnceLock;

pub struct FunctionDef {
    pub name: &'static str,
    pub func: fn(&[Number]) -> Result<Number, EngineError>,
}

inventory::collect!(FunctionDef);

static FUNCTION_REGISTRY: OnceLock<HashMap<&'static str, fn(&[Number]) -> Result<Number, EngineError>>> = OnceLock::new();

fn get_registry() -> &'static HashMap<&'static str, fn(&[Number]) -> Result<Number, EngineError>> {
    FUNCTION_REGISTRY.get_or_init(|| {
        let mut m = HashMap::new();
        for func_def in inventory::iter::<FunctionDef> {
            m.insert(func_def.name, func_def.func);
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
