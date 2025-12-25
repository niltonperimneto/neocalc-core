pub mod ast;
pub mod errors;
pub mod functions;
pub mod parser;
pub mod tokens;
pub mod types;

use crate::engine::errors::EngineError;
use crate::engine::types::Number;

use crate::engine::ast::Context;

pub fn evaluate(expression: &str, context: &mut Context) -> Result<Number, EngineError> {
    let expr = parser::parse(expression)?;
    expr.eval(context).map(|arc_num| (*arc_num).clone())
}
