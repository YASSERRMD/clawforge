//! Config redaction: produce safe-to-share config snapshots by masking sensitive fields.
//!
//! Masks API keys, tokens, phone numbers, bot tokens, and webhook secrets.

use regex::Regex;
use serde_json::Value;
use once_cell::sync::Lazy;

/// API key patterns: long alphanumeric strings that are secrets.
static API_KEY_KEYS: &[&str] = &[
    "apiKey",
    "api_key",
    "apikey",
    "botToken",
    "bot_token",
    "oauthToken",
    "oauth_token",
    "accessToken",
    "access_token",
    "webhookSecret",
    "webhook_secret",
    "channelSecret",
    "channel_secret",
    "channelAccessToken",
    "channel_access_token",
    "socketToken",
    "socket_token",
    "token",
    "secret",
    "password",
    "privateKey",
    "private_key",
];

/// Phone number pattern
static PHONE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\+?[0-9]{10,15}$").unwrap());

/// Redact a config JSON value, replacing all sensitive fields with `"***"`.
///
/// The resulting value is safe to log, display in the TUI, or share for debugging.
pub fn redact(value: &Value) -> Value {
    redact_recursive(value, "")
}

fn is_sensitive_key(key: &str) -> bool {
    API_KEY_KEYS.iter().any(|k| k.eq_ignore_ascii_case(key))
}

fn redact_string(s: &str, key: &str) -> Value {
    if is_sensitive_key(key) && !s.is_empty() {
        // Preserve length hint: show first 4 chars + ***
        let hint = if s.len() > 4 {
            format!("{}***", &s[..4])
        } else {
            "***".to_string()
        };
        return Value::String(hint);
    }

    // Redact phone numbers in any value
    if PHONE_PATTERN.is_match(s) && !s.is_empty() {
        let redacted = format!(
            "{}***",
            s.chars().take(4).collect::<String>()
        );
        return Value::String(redacted);
    }

    Value::String(s.to_string())
}

fn redact_recursive(value: &Value, key: &str) -> Value {
    match value {
        Value::String(s) => redact_string(s, key),
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| redact_recursive(v, key)).collect())
        }
        Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (k, v) in map {
                result.insert(k.clone(), redact_recursive(v, k));
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

/// Collect all field paths that were redacted (for diagnostics).
pub fn collect_redacted_paths(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    collect_paths_recursive(value, "", &mut paths);
    paths
}

fn collect_paths_recursive(value: &Value, path: &str, out: &mut Vec<String>) {
    match value {
        Value::String(s) if !s.is_empty() => {
            let key = path.rsplit('.').next().unwrap_or("");
            if is_sensitive_key(key) || PHONE_PATTERN.is_match(s) {
                out.push(path.to_string());
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                collect_paths_recursive(v, &format!("{path}[{i}]"), out);
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                collect_paths_recursive(v, &child_path, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_api_key() {
        let v = json!({ "auth": { "profiles": { "p1": { "apiKey": "sk-abcdef123456" } } } });
        let redacted = redact(&v);
        let key = &redacted["auth"]["profiles"]["p1"]["apiKey"];
        assert!(key.as_str().unwrap().ends_with("***"));
        assert!(!key.as_str().unwrap().contains("abcdef"));
    }

    #[test]
    fn redacts_bot_token() {
        let v = json!({ "channels": { "telegram": { "botToken": "1234567890:ABCDEF" } } });
        let redacted = redact(&v);
        let token = &redacted["channels"]["telegram"]["botToken"];
        assert!(token.as_str().unwrap().ends_with("***"));
    }

    #[test]
    fn passthrough_non_sensitive() {
        let v = json!({ "logging": { "level": "debug" } });
        let redacted = redact(&v);
        assert_eq!(redacted["logging"]["level"], "debug");
    }

    #[test]
    fn redacts_phone_in_allowfrom() {
        let v = json!({ "allowFrom": ["+12025551234"] });
        let redacted = redact(&v);
        let phone = &redacted["allowFrom"][0];
        assert!(phone.as_str().unwrap().ends_with("***"));
    }
}
