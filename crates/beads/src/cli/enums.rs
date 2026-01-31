//! Enum types for CLI argument parsing
//!
//! Provides typed enums with clap ValueEnum integration for automatic validation
//! and helpful error messages. Maintains backward compatibility with numeric priority values.

use std::fmt;
use std::str::FromStr;

/// Priority levels for issues
/// Maps user-friendly strings and numeric values to internal u32 values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(clap::ValueEnum)]
pub enum Priority {
    /// Low priority (0)
    Low,
    /// Medium priority (1) - default
    Medium,
    /// High priority (2)
    High,
    /// Urgent priority (3)
    Urgent,
}

impl Priority {
    /// Get the numeric value for storage
    pub fn as_u32(self) -> u32 {
        match self {
            Priority::Low => 0,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Urgent => 3,
        }
    }

    /// Parse from string, accepting both names and numbers for backward compatibility
    pub fn from_str_compat(s: &str) -> Option<Self> {
        // Try parsing as name first (case-insensitive)
        match s.to_lowercase().as_str() {
            "low" => Some(Priority::Low),
            "medium" => Some(Priority::Medium),
            "high" => Some(Priority::High),
            "urgent" => Some(Priority::Urgent),
            _ => {
                // Try parsing as number
                match s.parse::<u32>() {
                    Ok(0) => Some(Priority::Low),
                    Ok(1) => Some(Priority::Medium),
                    Ok(2) => Some(Priority::High),
                    Ok(3) => Some(Priority::Urgent),
                    _ => None,
                }
            }
        }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(clap::ValueEnum)]
pub enum Kind {
    /// Bug fix
    Bug,
    /// New feature
    Feature,
    /// Code refactoring
    Refactor,
    /// Documentation
    Docs,
    /// Maintenance task
    Chore,
    /// General task
    Task,
}

impl Kind {
    /// Get the string value for storage
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(clap::ValueEnum)]
pub enum Status {
    /// Issue is open and ready to work on
    Open,
    /// Issue is being worked on
    #[value(alias = "in-progress", alias = "inprogress")]
    InProgress,
    /// Issue is ready for review
    Review,
    /// Issue is closed
    Closed,
}

impl Status {
    /// Get the string value for storage
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
    fn test_priority_from_str_compat() {
        // Names (case insensitive)
        assert_eq!(Priority::from_str_compat("low"), Some(Priority::Low));
        assert_eq!(Priority::from_str_compat("LOW"), Some(Priority::Low));
        assert_eq!(Priority::from_str_compat("Low"), Some(Priority::Low));
        assert_eq!(Priority::from_str_compat("medium"), Some(Priority::Medium));
        assert_eq!(Priority::from_str_compat("high"), Some(Priority::High));
        assert_eq!(Priority::from_str_compat("urgent"), Some(Priority::Urgent));

        // Numbers (backward compat)
        assert_eq!(Priority::from_str_compat("0"), Some(Priority::Low));
        assert_eq!(Priority::from_str_compat("1"), Some(Priority::Medium));
        assert_eq!(Priority::from_str_compat("2"), Some(Priority::High));
        assert_eq!(Priority::from_str_compat("3"), Some(Priority::Urgent));

        // Invalid
        assert_eq!(Priority::from_str_compat("critical"), None);
        assert_eq!(Priority::from_str_compat("5"), None);
    }

    #[test]
    fn test_priority_as_u32() {
        assert_eq!(Priority::Low.as_u32(), 0);
        assert_eq!(Priority::Medium.as_u32(), 1);
        assert_eq!(Priority::High.as_u32(), 2);
        assert_eq!(Priority::Urgent.as_u32(), 3);
    }

    #[test]
    fn test_kind_as_str() {
        assert_eq!(Kind::Bug.as_str(), "bug");
        assert_eq!(Kind::Feature.as_str(), "feature");
        assert_eq!(Kind::Task.as_str(), "task");
    }

    #[test]
    fn test_status_as_str() {
        assert_eq!(Status::Open.as_str(), "open");
        assert_eq!(Status::InProgress.as_str(), "in_progress");
        assert_eq!(Status::Closed.as_str(), "closed");
    }
}
