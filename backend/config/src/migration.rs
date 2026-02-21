//! Legacy config migration engine.
//!
//! Applies sequential migrations to bring old config formats up to the current schema.
//! Each migration is versioned and idempotent.

use anyhow::Result;
use serde_json::Value;
use tracing::info;

/// Current config schema version. Configs with lower versions will be migrated.
pub const CURRENT_VERSION: u32 = 4;

/// Apply all pending migrations to the config JSON value.
/// The `version` field in the config indicates which migrations have already run.
pub fn migrate(mut value: Value, from_version: u32) -> Result<(Value, bool)> {
    let mut mutated = false;
    let mut current = from_version;

    if current < 2 {
        value = migrate_v1_to_v2(value)?;
        current = 2;
        mutated = true;
        info!("Migrated config from v1 → v2");
    }

    if current < 3 {
        value = migrate_v2_to_v3(value)?;
        current = 3;
        mutated = true;
        info!("Migrated config from v2 → v3");
    }

    if current < 4 {
        value = migrate_v3_to_v4(value)?;
        mutated = true;
        info!("Migrated config from v3 → v4");
    }

    Ok((value, mutated))
}

/// v1 → v2: Rename `routing.allowFrom` → channel-specific `allowFrom` fields.
///
/// Old format: `{ routing: { allowFrom: ["..."] } }`
/// New format: each channel's own `allowFrom` list
fn migrate_v1_to_v2(mut value: Value) -> Result<Value> {
    if let Some(routing) = value.get("routing").cloned() {
        if let Some(allow_from) = routing.get("allowFrom") {
            // Propagate to whatsapp and telegram channels if not already set
            for channel in ["whatsapp", "telegram", "discord", "slack"] {
                let channels = value
                    .get_mut("channels")
                    .and_then(|c| c.get_mut(channel));
                if let Some(ch) = channels {
                    if ch.get("allowFrom").is_none() {
                        if let Value::Object(ch_map) = ch {
                            ch_map.insert("allowFrom".to_string(), allow_from.clone());
                        }
                    }
                }
            }
            // Remove old routing field
            if let Value::Object(map) = &mut value {
                map.remove("routing");
            }
        }
    }
    Ok(value)
}

/// v2 → v3: Rename `session.mainKey` → removed (always "main").
///
/// Also renames `channels.whatsapp.dmPolicy` field aliases for Slack/Discord.
fn migrate_v2_to_v3(mut value: Value) -> Result<Value> {
    // Remove deprecated session.mainKey
    if let Value::Object(map) = &mut value {
        if let Some(session) = map.get_mut("session") {
            if let Value::Object(session_map) = session {
                session_map.remove("mainKey");
            }
        }
    }

    // Rename dmPolicy aliases: "groups-only" → "groups" for Slack/Discord
    for channel in ["slack", "discord"] {
        if let Some(ch) = value
            .get_mut("channels")
            .and_then(|c| c.get_mut(channel))
        {
            if let Value::Object(ch_map) = ch {
                if let Some(policy) = ch_map.get("dmPolicy") {
                    if policy == "groups-only" {
                        ch_map.insert("dmPolicy".to_string(), Value::String("groups".to_string()));
                    }
                }
            }
        }
    }

    Ok(value)
}

/// v3 → v4: Standardize `auth.profiles` — ensure every profile has `provider` field.
///
/// Old configs sometimes had provider inferred from key prefix (e.g., `anthropic-1` → anthropic).
fn migrate_v3_to_v4(mut value: Value) -> Result<Value> {
    if let Some(profiles) = value
        .get_mut("auth")
        .and_then(|a| a.get_mut("profiles"))
    {
        if let Value::Object(profiles_map) = profiles {
            for (key, profile) in profiles_map.iter_mut() {
                if let Value::Object(p) = profile {
                    if p.get("provider").is_none() {
                        // Infer provider from key prefix
                        let provider = if key.starts_with("anthropic") {
                            "anthropic"
                        } else if key.starts_with("openai") {
                            "openai"
                        } else if key.starts_with("google") {
                            "google"
                        } else if key.starts_with("ollama") {
                            "ollama"
                        } else {
                            continue;
                        };
                        p.insert(
                            "provider".to_string(),
                            Value::String(provider.to_string()),
                        );
                    }
                }
            }
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_migration_needed_at_current_version() {
        let (_, mutated) = migrate(json!({}), CURRENT_VERSION).unwrap();
        assert!(!mutated);
    }

    #[test]
    fn migrates_routing_allowfrom_v1() {
        let cfg = json!({
            "routing": { "allowFrom": ["+1234567890"] }
        });
        let (result, mutated) = migrate(cfg, 1).unwrap();
        assert!(mutated);
        assert!(result.get("routing").is_none(), "routing should be removed");
    }

    #[test]
    fn migrates_session_main_key_v2() {
        let cfg = json!({
            "session": { "mainKey": "my-session" }
        });
        let (result, mutated) = migrate(cfg, 2).unwrap();
        assert!(mutated);
        let main_key = result
            .get("session")
            .and_then(|s| s.get("mainKey"));
        assert!(main_key.is_none(), "mainKey should be removed");
    }
}
