//! Log Redaction Layer
//!
//! Scrubs API keys, access tokens, and phone numbers from strings prior to logging.

use regex::Regex;
use std::sync::LazyLock;

static TELEPHONE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}").unwrap());
static API_KEY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(sk-[a-zA-Z0-9]{32,})|(Bearer\s+[a-zA-Z0-9\-\._~+/]+=*)").unwrap());

/// Redacts sensitive patterns in a string.
pub fn redact_sensitive_data(input: &str) -> String {
    let mut redacted = input.to_string();
    
    // Redact telephone numbers
    redacted = TELEPHONE_RE.replace_all(&redacted, "[REDACTED_PHONE]").to_string();
    
    // Redact API keys and bearer tokens
    redacted = API_KEY_RE.replace_all(&redacted, "[REDACTED_TOKEN]").to_string();

    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redaction() {
        let raw = "Sending to +1-555-123-4567 with Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let clean = redact_sensitive_data(raw);
        assert!(!clean.contains("+1-555-123-4567"));
        assert!(!clean.contains("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }
}
