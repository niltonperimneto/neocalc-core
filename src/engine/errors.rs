use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum EngineError {
    #[error("Division by zero")]
    DivisionByZero,

    #[error("Math error: {0}")]
    DomainError(String),

    #[error("Undefined variable '{0}' — define it first, e.g. '{0} = 1'")]
    UndefinedVariable(String),

    #[error("Function '{name}' expects {expected}, but got {got}")]
    ArgumentMismatch {
        name: String,
        /// Human-readable description of the accepted arity, e.g. "1 argument"
        /// or "3 to 5 arguments".
        expected: String,
        /// The number of arguments actually supplied.
        got: usize,
    },

    #[error("Unknown function '{0}' — check the spelling")]
    UnknownFunction(String),

    #[error("Type error: expected {expected}, but got {got}")]
    TypeMismatch { expected: String, got: String },

    #[error("Syntax error: {0}")]
    ParserError(String),

    #[error("Computation too large: {0}")]
    ResourceLimit(String),

    #[error("{0}")]
    Generic(String),
}

impl EngineError {
    /// Convenience constructor for an exact-arity mismatch.
    pub fn arity(name: impl Into<String>, expected: usize, got: usize) -> Self {
        EngineError::ArgumentMismatch {
            name: name.into(),
            expected: format!(
                "{} argument{}",
                expected,
                if expected == 1 { "" } else { "s" }
            ),
            got,
        }
    }
}

impl From<String> for EngineError {
    fn from(s: String) -> Self {
        EngineError::Generic(s)
    }
}
