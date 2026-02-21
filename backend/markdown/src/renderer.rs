//! Renderers for the Markdown IR
//!
//! Transforms the AST into text (TTS), ANSI (Terminal), HTML, and WhatsApp syntax.

use crate::ir::MarkdownNode;

pub struct Renderer;

impl Renderer {
    /// Renders AST to plain text, stripping all formatting. Ideal for TTS engines.
    pub fn to_plain_text(nodes: &[MarkdownNode]) -> String {
        let mut output = String::new();
        for node in nodes {
            match node {
                MarkdownNode::Text(text) => output.push_str(text),
                MarkdownNode::Paragraph(children) => {
                    output.push_str(&Self::to_plain_text(children));
                    output.push('\n');
                }
                MarkdownNode::CodeBlock(_, content) => {
                    output.push_str("Code Example:\n");
                    output.push_str(content);
                    output.push('\n');
                }
                _ => {} // Handle others recursively
            }
        }
        output
    }

    /// Renders AST to ANSI terminal codes (Mock).
    pub fn to_ansi(nodes: &[MarkdownNode]) -> String {
        // MOCK: Colorize output
        Self::to_plain_text(nodes)
    }

    /// Renders AST to WhatsApp compatible markdown.
    pub fn to_whatsapp(nodes: &[MarkdownNode]) -> String {
        // MOCK: Replace ** with * and map formatting strictly to WA syntax
        Self::to_plain_text(nodes)
    }
}
