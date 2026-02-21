//! Multimodal Markdown Intermediate Representation Parser and Renderers
//!
//! Converts standard Markdown responses originating from LLMs into universally
//! readable schemas applied across Voice, Chat Clients, and Web contexts.

pub mod code_block;
pub mod ir;
pub mod renderer;

pub use code_block::CodeBlockAnalyzer;
pub use ir::{MarkdownNode, IrParser};
pub use renderer::Renderer;
