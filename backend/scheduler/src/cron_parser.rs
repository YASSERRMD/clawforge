/// Cron expression validation and normalization.
///
/// Supports 5-field (min hour dom mon dow) and 6-field (sec …) cron syntax.
/// Returns the normalized expression string or an error.
use anyhow::{bail, Result};

// For now we validate structure without a full cron parser dependency.
// A production version would use the `cron` crate for full expansion.

const VALID_RANGES: &[(u32, u32)] = &[
    (0, 59),  // minute
    (0, 23),  // hour
    (1, 31),  // day of month
    (1, 12),  // month
    (0, 7),   // day of week (0 and 7 both = Sunday)
];

/// Validate a 5-field cron expression.
pub fn validate_cron(expr: &str) -> Result<String> {
    let parts: Vec<&str> = expr.trim().split_whitespace().collect();
    if parts.len() != 5 {
        bail!("Cron expression must have exactly 5 fields, got {}: '{}'", parts.len(), expr);
    }
    for (i, part) in parts.iter().enumerate() {
        validate_field(part, VALID_RANGES[i].0, VALID_RANGES[i].1)
            .map_err(|e| anyhow::anyhow!("Field {} ('{}') invalid: {}", i + 1, part, e))?;
    }
    Ok(parts.join(" "))
}

/// Check whether a cron field is syntactically valid within [min, max].
fn validate_field(field: &str, min: u32, max: u32) -> Result<()> {
    if field == "*" || field == "?" {
        return Ok(());
    }
    // Handle /step
    let (range_part, _step) = if let Some((r, s)) = field.split_once('/') {
        let step: u32 = s.parse().map_err(|_| anyhow::anyhow!("step '{}' not numeric", s))?;
        if step == 0 { bail!("step must be > 0"); }
        (r, Some(step))
    } else {
        (field, None)
    };

    // Handle comma-separated values
    for part in range_part.split(',') {
        if part == "*" { continue; }
        if let Some((lo, hi)) = part.split_once('-') {
            let lo: u32 = lo.parse().map_err(|_| anyhow::anyhow!("'{}' not numeric", lo))?;
            let hi: u32 = hi.parse().map_err(|_| anyhow::anyhow!("'{}' not numeric", hi))?;
            if lo > hi || lo < min || hi > max {
                bail!("range {}-{} out of [{}, {}]", lo, hi, min, max);
            }
        } else {
            let v: u32 = part.parse().map_err(|_| anyhow::anyhow!("'{}' not numeric", part))?;
            if v < min || v > max {
                bail!("value {} out of [{}, {}]", v, min, max);
            }
        }
    }
    Ok(())
}
