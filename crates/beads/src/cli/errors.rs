//! Error message helpers for CLI
//!
//! Provides user-friendly error messages for invalid enum values and other CLI errors.

/// Generate an error message for invalid enum values
///
/// # Arguments
/// * `field` - The field name (e.g., "priority")
/// * `value` - The invalid value provided by the user
/// * `valid_values` - List of valid values to display
///
/// # Example
/// ```
/// let msg = invalid_enum_error("priority", "critical", &["low", "medium", "high", "urgent"]);
/// // Returns: "Invalid priority value 'critical'\n  Valid values: low, medium, high, urgent\n  Example: --priority low"
/// ```
#[allow(dead_code)]
pub fn invalid_enum_error(field: &str, value: &str, valid_values: &[&str]) -> String {
    format!(
        "Invalid {} value '{}'\n  Valid values: {}\n  Example: --{} {}",
        field,
        value,
        valid_values.join(", "),
        field,
        valid_values.first().unwrap_or(&"<value>")
    )
}

/// Generate a short error message for invalid enum values
///
/// Use this when the full context is already clear from surrounding output.
pub fn invalid_enum_error_short(field: &str, value: &str, valid_values: &[&str]) -> String {
    format!(
        "Invalid {} '{}'. Valid values: {}",
        field,
        value,
        valid_values.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_enum_error() {
        let msg = invalid_enum_error("priority", "critical", &["low", "medium", "high", "urgent"]);
        assert!(msg.contains("Invalid priority value 'critical'"));
        assert!(msg.contains("Valid values: low, medium, high, urgent"));
        assert!(msg.contains("Example: --priority low"));
    }

    #[test]
    fn test_invalid_enum_error_short() {
        let msg = invalid_enum_error_short("priority", "critical", &["low", "medium", "high", "urgent"]);
        assert!(msg.contains("Invalid priority 'critical'"));
        assert!(msg.contains("Valid values: low, medium, high, urgent"));
    }
}
