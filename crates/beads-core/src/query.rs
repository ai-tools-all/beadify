use chrono::DateTime;
use chrono_tz::Tz;

use crate::error::Result;

/// Parse date expression (relative or absolute) in user's timezone
/// Wrapper around tz module functions that returns UTC timestamp string
pub fn parse_date(date_str: &str, user_tz: Tz) -> Result<String> {
    crate::tz::parse_date_in_timezone(date_str, user_tz)
}

/// Parse relative date expression like "1 week ago" in user's timezone
pub fn parse_relative_date(expr: &str, user_tz: Tz) -> Result<String> {
    crate::tz::parse_relative_in_timezone(expr, user_tz)
}

/// Parse absolute date like "2026-01-20" in user's timezone
pub fn parse_absolute_date(date_str: &str, user_tz: Tz) -> Result<String> {
    crate::tz::parse_absolute_in_timezone(date_str, user_tz)
}

/// Check if issue was created after given UTC timestamp
pub fn created_after(issue_created_at: &str, after_date: &str) -> bool {
    if let (Ok(issue_dt), Ok(after_dt)) = (
        DateTime::parse_from_rfc3339(issue_created_at),
        DateTime::parse_from_rfc3339(after_date),
    ) {
        issue_dt >= after_dt
    } else {
        false
    }
}

/// Check if issue was created before given UTC timestamp
pub fn created_before(issue_created_at: &str, before_date: &str) -> bool {
    if let (Ok(issue_dt), Ok(before_dt)) = (
        DateTime::parse_from_rfc3339(issue_created_at),
        DateTime::parse_from_rfc3339(before_date),
    ) {
        issue_dt <= before_dt
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relative_date_days_ago() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_relative_date("3 days ago", tz).unwrap();
        // Should return valid RFC3339 date
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_relative_date_weeks_ago() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_relative_date("1 week ago", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_absolute_date_iso() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_absolute_date("2026-01-20T10:00:00Z", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_absolute_date_only() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_absolute_date("2026-01-20", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_date_auto_detects_relative() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_date("1 week ago", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_date_auto_detects_absolute() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_date("2026-01-20", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_created_after() {
        let issue_date = "2026-01-25T10:00:00Z";
        let after_date = "2026-01-20T00:00:00Z";
        assert!(created_after(issue_date, after_date));
    }

    #[test]
    fn test_created_before() {
        let issue_date = "2026-01-15T10:00:00Z";
        let before_date = "2026-01-20T00:00:00Z";
        assert!(created_before(issue_date, before_date));
    }
}
