//! Enum types for CLI argument parsing
//!
//! Provides typed enums with clap ValueEnum integration for automatic validation
//! and helpful error messages. Maintains backward compatibility with numeric priority values.

use std::fmt;

/// Priority levels for issues
/// Maps user-friendly strings and numeric values to internal u32 values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl clap::ValueEnum for Priority {
    fn value_variants<'a>() -> &'a [Self] {
        &[Priority::Low, Priority::Medium, Priority::High, Priority::Urgent]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Priority::Low => clap::builder::PossibleValue::new("low")
                .alias("LOW")
                .alias("Low")
                .alias("0")
                .help("Low priority (0)"),
            Priority::Medium => clap::builder::PossibleValue::new("medium")
                .alias("MEDIUM")
                .alias("Medium")
                .alias("1")
                .help("Medium priority (1) - default"),
            Priority::High => clap::builder::PossibleValue::new("high")
                .alias("HIGH")
                .alias("High")
                .alias("2")
                .help("High priority (2)"),
            Priority::Urgent => clap::builder::PossibleValue::new("urgent")
                .alias("URGENT")
                .alias("Urgent")
                .alias("3")
                .help("Urgent priority (3)"),
        })
    }
}

/// Issue kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl clap::ValueEnum for Kind {
    fn value_variants<'a>() -> &'a [Self] {
        &[Kind::Bug, Kind::Feature, Kind::Refactor, Kind::Docs, Kind::Chore, Kind::Task]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Kind::Bug => clap::builder::PossibleValue::new("bug")
                .alias("BUG")
                .alias("Bug")
                .help("Bug fix"),
            Kind::Feature => clap::builder::PossibleValue::new("feature")
                .alias("FEATURE")
                .alias("Feature")
                .help("New feature"),
            Kind::Refactor => clap::builder::PossibleValue::new("refactor")
                .alias("REFACTOR")
                .alias("Refactor")
                .help("Code refactoring"),
            Kind::Docs => clap::builder::PossibleValue::new("docs")
                .alias("DOCS")
                .alias("Docs")
                .help("Documentation"),
            Kind::Chore => clap::builder::PossibleValue::new("chore")
                .alias("CHORE")
                .alias("Chore")
                .help("Maintenance task"),
            Kind::Task => clap::builder::PossibleValue::new("task")
                .alias("TASK")
                .alias("Task")
                .help("General task"),
        })
    }
}

/// Issue statuses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Issue is open and ready to work on
    Open,
    /// Issue is being worked on
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

impl clap::ValueEnum for Status {
    fn value_variants<'a>() -> &'a [Self] {
        &[Status::Open, Status::InProgress, Status::Review, Status::Closed]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Status::Open => clap::builder::PossibleValue::new("open")
                .alias("OPEN")
                .alias("Open")
                .help("Issue is open and ready to work on"),
            Status::InProgress => clap::builder::PossibleValue::new("in-progress")
                .alias("IN-PROGRESS")
                .alias("In-Progress")
                .alias("in_progress")
                .alias("IN_PROGRESS")
                .alias("In_Progress")
                .alias("inprogress")
                .alias("INPROGRESS")
                .alias("InProgress")
                .help("Issue is being worked on"),
            Status::Review => clap::builder::PossibleValue::new("review")
                .alias("REVIEW")
                .alias("Review")
                .help("Issue is ready for review"),
            Status::Closed => clap::builder::PossibleValue::new("closed")
                .alias("CLOSED")
                .alias("Closed")
                .help("Issue is closed"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
