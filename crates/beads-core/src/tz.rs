use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use std::str::FromStr;

use crate::error::Result;

/// Get user's timezone from priority order:
/// 1. Explicit parameter (from CLI --timezone flag)
/// 2. TZ environment variable
/// 3. System default timezone
/// 4. UTC fallback
pub fn get_user_timezone(explicit_tz: Option<&str>) -> Result<Tz> {
    // 1. CLI flag override
    if let Some(tz_str) = explicit_tz {
        return Tz::from_str(tz_str).map_err(|_| crate::error::Error::Other {
            message: format!("Invalid timezone: {}", tz_str),
        });
    }

    // 2. TZ environment variable
    if let Ok(tz_str) = std::env::var("TZ") {
        return Tz::from_str(&tz_str).map_err(|_| crate::error::Error::Other {
            message: format!("Invalid TZ env var: {}", tz_str),
        });
    }

    // 3. System default timezone detection
    #[cfg(unix)]
    {
        // Try /etc/timezone
        if let Ok(tz_str) = std::fs::read_to_string("/etc/timezone") {
            let tz_name = tz_str.trim();
            if let Ok(tz) = Tz::from_str(tz_name) {
                return Ok(tz);
            }
        }

        // Try symlink /etc/localtime
        if let Ok(link) = std::fs::read_link("/etc/localtime") {
            if let Some(path_str) = link.to_str() {
                // Path looks like: /usr/share/zoneinfo/America/New_York
                if let Some(pos) = path_str.find("zoneinfo/") {
                    let tz_name = &path_str[pos + 9..]; // Skip "zoneinfo/"
                    if let Ok(tz) = Tz::from_str(tz_name) {
                        return Ok(tz);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Try Windows Registry or system API
        // Fallback to UTC if not available
    }

    // 4. Fallback to UTC
    Ok(Tz::UTC)
}

/// Convert UTC timestamp to user's local timezone
/// Returns formatted string: "2026-01-31 15:30 EST"
pub fn utc_to_local_string(utc_timestamp: &str, user_tz: Tz) -> Result<String> {
    let utc_dt =
        DateTime::parse_from_rfc3339(utc_timestamp).map_err(|e| crate::error::Error::Other {
            message: format!("Failed to parse timestamp: {}", e),
        })?;

    let local_dt = utc_dt.with_timezone(&user_tz);
    let tz_name = user_tz.name();

    Ok(format!("{} {}", local_dt.format("%Y-%m-%d %H:%M"), tz_name))
}

/// Parse relative date expression in user's local timezone and return UTC timestamp
/// Examples: "1 week ago", "3 days ago", "2 months ago"
pub fn parse_relative_in_timezone(expr: &str, user_tz: Tz) -> Result<String> {
    let expr = expr.to_lowercase();
    let parts: Vec<&str> = expr.split_whitespace().collect();

    if parts.len() < 3 || parts[parts.len() - 1] != "ago" {
        return Err(crate::error::Error::Other {
            message: "Expected format: 'N days ago' or 'N weeks ago'".to_string(),
        });
    }

    let num: i64 = parts[0].parse().map_err(|_| crate::error::Error::Other {
        message: format!("Invalid number: {}", parts[0]),
    })?;

    let unit = parts[parts.len() - 2];

    // Get current time in user's timezone
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&user_tz);

    // Calculate target time in user's timezone
    let target_local = match unit {
        "day" | "days" => now_local - Duration::days(num),
        "week" | "weeks" => now_local - Duration::weeks(num),
        "month" | "months" => now_local - Duration::days(num * 30),
        "year" | "years" => now_local - Duration::days(num * 365),
        _ => {
            return Err(crate::error::Error::Other {
                message: format!(
                    "Unknown time unit: {}. Use days, weeks, months, or years",
                    unit
                ),
            })
        }
    };

    // Convert back to UTC for storage
    let target_utc = target_local.with_timezone(&Utc);
    Ok(target_utc.to_rfc3339())
}

/// Parse absolute date in user's local timezone
/// Assumes midnight (00:00) in user's timezone
/// Examples: "2026-01-20", "2026-01-20T15:30:00"
pub fn parse_absolute_in_timezone(date_str: &str, user_tz: Tz) -> Result<String> {
    // Try ISO 8601 with time
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.to_rfc3339());
    }

    // Try date-only format YYYY-MM-DD (assume midnight local time)
    if let Ok(naive_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
        let local_dt = user_tz
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| crate::error::Error::Other {
                message: format!("Ambiguous or invalid local datetime: {}", date_str),
            })?;
        return Ok(local_dt.with_timezone(&Utc).to_rfc3339());
    }

    Err(crate::error::Error::Other {
        message: format!(
            "Invalid date format: {}. Use YYYY-MM-DD or ISO 8601",
            date_str
        ),
    })
}

/// Parse either relative or absolute date expression
pub fn parse_date_in_timezone(date_str: &str, user_tz: Tz) -> Result<String> {
    if date_str.contains("ago") {
        parse_relative_in_timezone(date_str, user_tz)
    } else {
        parse_absolute_in_timezone(date_str, user_tz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_get_user_timezone_utc_fallback() {
        let tz = get_user_timezone(None).unwrap();
        // Should return UTC or system default
        assert!(!tz.name().is_empty());
    }

    #[test]
    fn test_get_user_timezone_explicit() {
        let tz = get_user_timezone(Some("America/New_York")).unwrap();
        assert_eq!(tz, Tz::America__New_York);
    }

    #[test]
    fn test_utc_to_local_string() {
        let utc_ts = "2026-01-31T20:00:00Z";
        let est = Tz::America__New_York;
        let local = utc_to_local_string(utc_ts, est).unwrap();
        assert!(local.contains("2026-01-31"));
        assert!(local.contains("America/New_York"));
    }

    #[test]
    fn test_parse_relative_in_timezone() {
        let tz = Tz::UTC;
        let result = parse_relative_in_timezone("1 week ago", tz).unwrap();
        let dt = DateTime::parse_from_rfc3339(&result).unwrap();
        let now = Utc::now();
        // Should be approximately 7 days ago
        let diff = now.signed_duration_since(dt);
        assert!(diff.num_days() >= 6 && diff.num_days() <= 8);
    }

    #[test]
    fn test_parse_absolute_in_timezone() {
        let tz = Tz::UTC;
        let result = parse_absolute_in_timezone("2026-01-20", tz).unwrap();
        let dt = DateTime::parse_from_rfc3339(&result).unwrap();
        // Should be 2026-01-20 at 00:00 UTC
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 20);
    }

    #[test]
    fn test_parse_date_auto_detects_relative() {
        let tz = Tz::UTC;
        let result = parse_date_in_timezone("1 week ago", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_date_auto_detects_absolute() {
        let tz = Tz::UTC;
        let result = parse_date_in_timezone("2026-01-20", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }
}
