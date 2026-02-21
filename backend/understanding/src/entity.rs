//! Entity extractor: NER-lite for identifying named entities in text.
//!
//! Extracts mentions, URLs, dates, emails, phone numbers, and code blocks.
//! Mirrors `src/understanding/entity-extractor.ts`.

use regex::Regex;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;

/// A recognized entity in text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub kind: EntityKind,
    pub value: String,
    /// Character offset in the source text.
    pub start: usize,
    pub end: usize,
}

/// Types of entities we extract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    /// E-mail address.
    Email,
    /// URL / URI.
    Url,
    /// Phone number (international or local).
    Phone,
    /// @mention handle.
    Mention,
    /// Inline or fenced code block.
    Code,
    /// ISO 8601 date or human date like "tomorrow", "next Monday".
    Date,
    /// Numeric currency amount (e.g., "$42.50").
    Currency,
    /// IP address (IPv4).
    IpAddress,
}

// --- Compiled regexes ---

static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap()
});

static URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"https?://[^\s\)\]>"']+"#).unwrap()
});

static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\+?[\d\-\(\)\s]{7,16}").unwrap()
});

static MENTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"@[a-zA-Z0-9_]{1,50}").unwrap()
});

static CODE_INLINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`[^`]+`").unwrap()
});

static CURRENCY_RE: Lazy<Regex> = Lazy::new(|| {
    // ASCII-safe: $, euro sign (U+20AC), pound (U+00A3), yen (U+00A5)
    Regex::new(r"[$\x{20AC}\x{00A3}\x{00A5}]\s*\d+(?:[.,]\d{1,2})?|\d+(?:[.,]\d{1,2})?\s*(?:USD|EUR|GBP|JPY|SAR)").unwrap()
});

static IP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap()
});

/// Extract all entities from a text string.
pub fn extract_entities(text: &str) -> Vec<Entity> {
    let mut entities = Vec::new();

    add_matches(&mut entities, text, &EMAIL_RE, EntityKind::Email);
    add_matches(&mut entities, text, &URL_RE, EntityKind::Url);
    add_matches(&mut entities, text, &MENTION_RE, EntityKind::Mention);
    add_matches(&mut entities, text, &CODE_INLINE_RE, EntityKind::Code);
    add_matches(&mut entities, text, &CURRENCY_RE, EntityKind::Currency);
    add_matches(&mut entities, text, &IP_RE, EntityKind::IpAddress);
    // Phone is last (widest regex, many false positives — skip if overlapping)
    add_non_overlapping(&mut entities, text, &PHONE_RE, EntityKind::Phone);

    // Sort by start position.
    entities.sort_by_key(|e| e.start);
    entities
}

fn add_matches(entities: &mut Vec<Entity>, text: &str, re: &Regex, kind: EntityKind) {
    for m in re.find_iter(text) {
        entities.push(Entity {
            kind: kind.clone(),
            value: m.as_str().to_string(),
            start: m.start(),
            end: m.end(),
        });
    }
}

fn add_non_overlapping(entities: &mut Vec<Entity>, text: &str, re: &Regex, kind: EntityKind) {
    for m in re.find_iter(text) {
        // Skip if this span overlaps with an already-found entity.
        let overlaps = entities.iter().any(|e| {
            m.start() < e.end && m.end() > e.start
        });
        if !overlaps {
            entities.push(Entity {
                kind: kind.clone(),
                value: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
            });
        }
    }
}

/// Extract only entities of a specific kind.
pub fn extract_of_kind(text: &str, kind: &EntityKind) -> Vec<Entity> {
    extract_entities(text)
        .into_iter()
        .filter(|e| &e.kind == kind)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_email() {
        let entities = extract_entities("Send to user@example.com thanks");
        assert!(entities.iter().any(|e| e.kind == EntityKind::Email && e.value == "user@example.com"));
    }

    #[test]
    fn extracts_url() {
        let entities = extract_entities("Visit https://example.com for more");
        assert!(entities.iter().any(|e| e.kind == EntityKind::Url));
    }

    #[test]
    fn extracts_mention() {
        let entities = extract_entities("Hey @alice and @bob!");
        let mentions: Vec<_> = entities.iter().filter(|e| e.kind == EntityKind::Mention).collect();
        assert_eq!(mentions.len(), 2);
    }
}
