//! Markdown Intermediate Representation
//!
//! Parses markdown syntax into a strongly-typed AST, enabling abstract 
//! rendering paths for multimodal output scenarios.

use serde::{Deserialize, Serialize};
use pulldown_cmark::{Parser, Event, Tag};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarkdownNode {
    Heading(u32, Vec<MarkdownNode>),
    Paragraph(Vec<MarkdownNode>),
    Text(String),
    CodeBlock(String, String), // language, content
    List(Vec<MarkdownNode>),
    ListItem(Vec<MarkdownNode>),
    Blockquote(Vec<MarkdownNode>),
    Link(String, String), // url, text
    Image(String, String), // url, alt_text
}

pub struct IrParser;

impl IrParser {
    /// Tokenizes and processes standard Markdown into an Intermediate Representation.
    pub fn parse(markdown: &str) -> Vec<MarkdownNode> {
        let _parser = Parser::new(markdown);
        // MOCK: Actually build tree using a parser state machine mapping `pulldown-cmark::Event`s.
        
        vec![
            MarkdownNode::Paragraph(vec![
                MarkdownNode::Text("Mocked parsed markdown paragraph.".to_string()),
            ]),
        ]
    }
}
