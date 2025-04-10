use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

/// Custom duration serialization/deserialization
/// Supports formats like "10s", "500ms", "1m", "1h"
pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format_duration(duration);
    s.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_duration(&s).map_err(serde::de::Error::custom)
}

/// Format a duration as a human-readable string
fn format_duration(duration: &Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if total_secs == 0 {
        return format!("{millis}ms");
    }

    if total_secs < 60 {
        if millis == 0 {
            format!("{total_secs}s")
        } else {
            format!("{}.{}s", total_secs, millis / 100)
        }
    } else if total_secs < 3600 {
        format!("{}m", total_secs / 60)
    } else {
        format!("{}h", total_secs / 3600)
    }
}

/// Parse a duration from a human-readable string
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty duration string".to_string());
    }

    // Extract numeric part and unit
    let (num_part, unit) = split_duration_string(s)?;

    // Parse the numeric part
    let value: f64 = num_part
        .parse()
        .map_err(|_| format!("Invalid number: {num_part}"))?;

    if value < 0.0 {
        return Err("Duration cannot be negative".to_string());
    }

    // Convert based on unit
    let duration = match unit {
        "ms" => Duration::from_millis(value as u64),
        "s" | "" => Duration::from_secs_f64(value),
        "m" => Duration::from_secs_f64(value * 60.0),
        "h" => Duration::from_secs_f64(value * 3600.0),
        _ => return Err(format!("Unknown time unit: {unit}")),
    };

    Ok(duration)
}

/// Split duration string into numeric part and unit
fn split_duration_string(s: &str) -> Result<(&str, &str), String> {
    // Find where the numeric part ends
    let boundary = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());

    if boundary == 0 {
        return Err("No numeric value found".to_string());
    }

    let num_part = &s[..boundary];
    let unit = &s[boundary..];

    Ok((num_part, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("10s").unwrap(), Duration::from_secs(10));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("0.5h").unwrap(), Duration::from_secs(1800));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(&Duration::from_secs(10)), "10s");
        assert_eq!(format_duration(&Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(&Duration::from_secs(120)), "2m");
        assert_eq!(format_duration(&Duration::from_secs(3600)), "1h");
    }
}
