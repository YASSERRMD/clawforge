//! Code Block Semantic Utility
//!
//! Detects source code language and filters specific programming artifacts.

use crate::ir::MarkdownNode;

pub struct CodeBlockAnalyzer;

impl CodeBlockAnalyzer {
    /// Extracts all code blocks from an AST.
    pub fn extract_blocks(nodes: &[MarkdownNode]) -> Vec<(String, String)> {
        let mut blocks = Vec::new();
        for node in nodes {
            if let MarkdownNode::CodeBlock(lang, content) = node {
                blocks.push((lang.clone(), content.clone()));
            }
            // Recurse mock
        }
        blocks
    }

    /// Replaces code blocks with a descriptive "code example" label for TTS pipelines.
    pub fn strip_for_tts(nodes: Vec<MarkdownNode>) -> Vec<MarkdownNode> {
        nodes.into_iter().map(|node| {
            match node {
                MarkdownNode::CodeBlock(lang, _) => {
                    MarkdownNode::Text(format!("[{} Code Example omitted for brevity]", lang))
                },
                _ => node, // Should recurse on children
            }
        }).collect()
    }
}
