pub mod browser;
pub mod compaction;
pub mod file;
pub mod loop_detection;
pub mod model_catalog;
pub mod node;
pub mod shell;
pub mod web;

pub use browser::BrowserTool;
pub use compaction::{compact_history, CompactionResult, Turn};
pub use file::{FileReadTool, FileWriteTool};
pub use loop_detection::{hash_input, LoopDetector, ToolCall};
pub use model_catalog::{ModelCatalog, ModelEntry};
pub use shell::ShellTool;
pub use web::{web_fetch, web_search, WebFetchInput, WebFetchOutput, WebSearchInput, WebSearchOutput, SearchHit};
