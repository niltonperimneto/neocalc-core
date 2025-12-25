use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum EngineError {
    #[error("Division by zero")]
    DivisionByZero,
    
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    
    #[error("Function '{0}' requires exactly {1} argument(s)")]
    ArgumentMismatch(String, usize),
    
    #[error("Function '{0}' is not known")]
    UnknownFunction(String),
    
    #[error("Type mismatch: expected {0}, got {1}")]
    TypeMismatch(String, String),
    
    #[error("Parser error: {0}")]
    ParserError(String),
    
    #[error("{0}")]
    Generic(String),
}

impl From<String> for EngineError {
    fn from(s: String) -> Self {
        EngineError::Generic(s)
    }
}
