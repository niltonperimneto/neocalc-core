pub mod engine;
pub mod i18n;
pub mod session_manager;
pub mod utils;
// Re-export common types for easier usage
pub use engine::ast::Context;
pub use engine::errors::EngineError;
pub use engine::evaluate;
pub use engine::types::Number;
pub use session_manager::{AppSessionManager, PersistError, Settings};

// We need to ensure the engine module is public and accessible
