//! Environment variable substitution and preservation for config values.
//!
//! Supports `${VAR_NAME}` syntax in string values, resolved at load time.
//! Only uppercase `[A-Z_][A-Z0-9_]*` variable names are matched.
//! `$${}` escapes to a literal `${}`.

use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Pattern matching valid uppercase env var names.
static ENV_VAR_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap());

/// Pattern matching escaped env var references (`$${}` → `${}`).
static ESCAPED_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\$\$\{([A-Z_][A-Z0-9_]*)\}").unwrap());

/// Error returned for missing env vars.
#[derive(Debug, thiserror::Error)]
#[error("Missing env var \"{var_name}\" referenced at config path: {config_path}")]
pub struct MissingEnvVarError {
    pub var_name: String,
    pub config_path: String,
}

/// Substitute `${VAR}` references in a config JSON value tree.
///
/// Walks the entire value tree recursively; only string leaves are processed.
/// Returns an error if any referenced env var is not set or is empty.
pub fn resolve_env_vars(value: &Value) -> Result<Value> {
    substitute_value(value, &std::env::vars().collect(), "")
}

/// Substitute env vars using a provided map (useful for testing).
pub fn resolve_env_vars_with(
    value: &Value,
    env: &HashMap<String, String>,
) -> Result<Value> {
    substitute_value(value, env, "")
}

fn substitute_value(
    value: &Value,
    env: &HashMap<String, String>,
    path: &str,
) -> Result<Value> {
    match value {
        Value::String(s) => {
            let substituted = substitute_string(s, env, path)?;
            Ok(Value::String(substituted))
        }
        Value::Array(arr) => {
            let result: Result<Vec<_>> = arr
                .iter()
                .enumerate()
                .map(|(i, v)| substitute_value(v, env, &format!("{path}[{i}]")))
                .collect();
            Ok(Value::Array(result?))
        }
        Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                result.insert(k.clone(), substitute_value(v, env, &child_path)?);
            }
            Ok(Value::Object(result))
        }
        // Primitives pass through unchanged.
        other => Ok(other.clone()),
    }
}

fn substitute_string(
    s: &str,
    env: &HashMap<String, String>,
    path: &str,
) -> Result<String> {
    if !s.contains('$') {
        return Ok(s.to_string());
    }

    // First handle escaped references: $${ → keep as ${
    let mut result = s.to_string();
    // Temporarily replace escaped refs before substitution pass
    let escaped: Vec<_> = ESCAPED_PATTERN.find_iter(s).map(|m| m.as_str().to_owned()).collect();

    // Now substitute real vars
    let mut error: Option<MissingEnvVarError> = None;
    let substituted = ENV_VAR_PATTERN.replace_all(&result, |caps: &regex::Captures| {
        if error.is_some() {
            return String::new();
        }
        let var_name = &caps[1];
        // Skip if this was an escaped ref position (heuristic: check $$)
        if let Some(start) = caps.get(0).map(|m| m.start()) {
            let bytes = result.as_bytes();
            if start > 0 && bytes.get(start.saturating_sub(1)) == Some(&b'$') {
                return caps[0].to_string();
            }
        }
        match env.get(var_name) {
            Some(val) if !val.is_empty() => val.clone(),
            _ => {
                error = Some(MissingEnvVarError {
                    var_name: var_name.to_string(),
                    config_path: path.to_string(),
                });
                String::new()
            }
        }
    });

    if let Some(err) = error {
        bail!(err);
    }

    // Restore escaped refs: $${ → ${
    let final_result = ESCAPED_PATTERN
        .replace_all(&substituted, |caps: &regex::Captures| {
            format!("${{{}}}", &caps[1])
        })
        .to_string();

    Ok(final_result)
}

/// Check whether a string contains any env var references.
pub fn contains_env_var_reference(s: &str) -> bool {
    s.contains('$') && ENV_VAR_PATTERN.is_match(s)
}

/// Collect all env var names referenced in a config value tree (for diagnostics).
pub fn collect_referenced_vars(value: &Value) -> Vec<String> {
    let mut vars = Vec::new();
    collect_vars_recursive(value, &mut vars);
    vars.sort();
    vars.dedup();
    vars
}

fn collect_vars_recursive(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            for caps in ENV_VAR_PATTERN.captures_iter(s) {
                out.push(caps[1].to_string());
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_vars_recursive(v, out);
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                collect_vars_recursive(v, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn substitutes_simple_var() {
        let v = json!({"apiKey": "${OPENAI_API_KEY}"});
        let env = env(&[("OPENAI_API_KEY", "sk-abc123")]);
        let result = resolve_env_vars_with(&v, &env).unwrap();
        assert_eq!(result["apiKey"], "sk-abc123");
    }

    #[test]
    fn error_on_missing_var() {
        let v = json!({"key": "${MISSING_VAR}"});
        let result = resolve_env_vars_with(&v, &HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MISSING_VAR"));
    }

    #[test]
    fn passthrough_non_var_strings() {
        let v = json!({"key": "plain-string"});
        let result = resolve_env_vars_with(&v, &HashMap::new()).unwrap();
        assert_eq!(result["key"], "plain-string");
    }

    #[test]
    fn substitutes_nested() {
        let v = json!({"a": {"b": "${MY_VAR}"}});
        let env = env(&[("MY_VAR", "hello")]);
        let result = resolve_env_vars_with(&v, &env).unwrap();
        assert_eq!(result["a"]["b"], "hello");
    }

    #[test]
    fn collects_referenced_vars() {
        let v = json!({"a": "${FOO}", "b": {"c": "${BAR}"}});
        let vars = collect_referenced_vars(&v);
        assert!(vars.contains(&"FOO".to_string()));
        assert!(vars.contains(&"BAR".to_string()));
    }
}
