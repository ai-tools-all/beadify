//! Issue subcommand module
//!
//! Provides natural CLI interface for issue management:
//! - `issue create` - Create a new issue with individual flags
//! - `issue update` - Update an existing issue
//! - `issue list` - List issues with filtering
//! - `issue show` - Show issue details

pub mod create;
pub mod list;
pub mod show;
pub mod update;
