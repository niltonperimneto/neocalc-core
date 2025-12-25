pub mod engine;
pub mod utils;

// Re-export common types for easier usage
pub use engine::ast::Context;
pub use engine::types::Number;
pub use engine::errors::EngineError;
pub use engine::evaluate;

// We need to ensure the engine module is public and accessible
