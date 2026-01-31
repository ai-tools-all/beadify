//! Enum types for CLI argument parsing
//!
//! Provides string-to-internal-value mappings for user-friendly CLI interfaces.
//! Uses match statements instead of external crates to minimize dependencies.

use std::fmt;

/// Priority levels for issues
/// Maps user-friendly strings to internal u32 values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
    Urgent = 3,
}

impl Priority {
    /// Parse priority from user-friendly string
    /// Returns Some(u32) if valid, None otherwise
    pub fn from_str(s: &str) -> Option<u32> {
        match s.to_lowercase().as_str() {
            "low" => Some(0),
            "medium" => Some(1),
            "high" => Some(2),
            "urgent" => Some(3),
            _ => None,
        }
    }

    /// Get all valid string values for help/error messages
    pub fn variants() -> Vec<&'static str> {
        vec!["low", "medium", "high", "urgent"]
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Urgent => write!(f, "urgent"),
        }
    }
}

/// Issue kinds
/// Maps user-friendly strings to internal string values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Kind {
    Bug,
    Feature,
    Refactor,
    Docs,
    Chore,
    Task,
}

impl Kind {
    /// Parse kind from user-friendly string
    /// Returns Some(String) if valid, None otherwise
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<String> {
        match s.to_lowercase().as_str() {
            "bug" => Some("bug".to_string()),
            "feature" => Some("feature".to_string()),
            "refactor" => Some("refactor".to_string()),
            "docs" => Some("docs".to_string()),
            "chore" => Some("chore".to_string()),
            "task" => Some("task".to_string()),
            _ => None,
        }
    }

    /// Get all valid string values for help/error messages
    #[allow(dead_code)]
    pub fn variants() -> Vec<&'static str> {
        vec!["bug", "feature", "refactor", "docs", "chore", "task"]
    }

    /// Get the string value
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Bug => "bug",
            Kind::Feature => "feature",
            Kind::Refactor => "refactor",
            Kind::Docs => "docs",
            Kind::Chore => "chore",
            Kind::Task => "task",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Issue statuses
/// Maps user-friendly strings to internal string values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Status {
    Open,
    InProgress,
    Review,
    Closed,
}

impl Status {
    /// Parse status from user-friendly string
    /// Returns Some(String) if valid, None otherwise
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<String> {
        match s.to_lowercase().as_str() {
            "open" => Some("open".to_string()),
            "in_progress" | "in-progress" | "inprogress" => Some("in_progress".to_string()),
            "review" => Some("review".to_string()),
            "closed" => Some("closed".to_string()),
            _ => None,
        }
    }

    /// Get all valid string values for help/error messages
    #[allow(dead_code)]
    pub fn variants() -> Vec<&'static str> {
        vec!["open", "in_progress", "review", "closed"]
    }

    /// Get the string value
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::InProgress => "in_progress",
            Status::Review => "review",
            Status::Closed => "closed",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_str() {
        assert_eq!(Priority::from_str("low"), Some(0));
        assert_eq!(Priority::from_str("medium"), Some(1));
        assert_eq!(Priority::from_str("high"), Some(2));
        assert_eq!(Priority::from_str("urgent"), Some(3));
        assert_eq!(Priority::from_str("LOW"), Some(0)); // case insensitive
        assert_eq!(Priority::from_str("invalid"), None);
    }

    #[test]
    fn test_priority_variants() {
        let variants = Priority::variants();
        assert!(variants.contains(&"low"));
        assert!(variants.contains(&"medium"));
        assert!(variants.contains(&"high"));
        assert!(variants.contains(&"urgent"));
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::Low.to_string(), "low");
        assert_eq!(Priority::Medium.to_string(), "medium");
        assert_eq!(Priority::High.to_string(), "high");
        assert_eq!(Priority::Urgent.to_string(), "urgent");
    }

    #[test]
    fn test_kind_from_str() {
        assert_eq!(Kind::from_str("bug"), Some("bug".to_string()));
        assert_eq!(Kind::from_str("feature"), Some("feature".to_string()));
        assert_eq!(Kind::from_str("refactor"), Some("refactor".to_string()));
        assert_eq!(Kind::from_str("docs"), Some("docs".to_string()));
        assert_eq!(Kind::from_str("chore"), Some("chore".to_string()));
        assert_eq!(Kind::from_str("task"), Some("task".to_string()));
        assert_eq!(Kind::from_str("BUG"), Some("bug".to_string())); // case insensitive
        assert_eq!(Kind::from_str("invalid"), None);
    }

    #[test]
    fn test_kind_variants() {
        let variants = Kind::variants();
        assert!(variants.contains(&"bug"));
        assert!(variants.contains(&"feature"));
        assert!(variants.contains(&"task"));
        assert_eq!(variants.len(), 6);
    }

    #[test]
    fn test_status_from_str() {
        assert_eq!(Status::from_str("open"), Some("open".to_string()));
        assert_eq!(Status::from_str("in_progress"), Some("in_progress".to_string()));
        assert_eq!(Status::from_str("in-progress"), Some("in_progress".to_string()));
        assert_eq!(Status::from_str("inprogress"), Some("in_progress".to_string()));
        assert_eq!(Status::from_str("review"), Some("review".to_string()));
        assert_eq!(Status::from_str("closed"), Some("closed".to_string()));
        assert_eq!(Status::from_str("OPEN"), Some("open".to_string())); // case insensitive
        assert_eq!(Status::from_str("invalid"), None);
    }

    #[test]
    fn test_status_variants() {
        let variants = Status::variants();
        assert!(variants.contains(&"open"));
        assert!(variants.contains(&"in_progress"));
        assert!(variants.contains(&"review"));
        assert!(variants.contains(&"closed"));
        assert_eq!(variants.len(), 4);
    }
}
