use chrono::{DateTime, Utc};

/// Format a timestamp as a human-readable relative time string.
/// Examples: "just now", "5 minutes ago", "2 hours ago", "3 days ago"
pub fn format_relative_time(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    let seconds = duration.num_seconds();
    if seconds < 0 {
        return "in the future".to_string();
    }

    if seconds < 60 {
        return "just now".to_string();
    }

    let minutes = duration.num_minutes();
    if minutes < 60 {
        if minutes == 1 {
            return "1 minute ago".to_string();
        }
        return format!("{} minutes ago", minutes);
    }

    let hours = duration.num_hours();
    if hours < 24 {
        if hours == 1 {
            return "1 hour ago".to_string();
        }
        return format!("{} hours ago", hours);
    }

    let days = duration.num_days();
    if days < 30 {
        if days == 1 {
            return "1 day ago".to_string();
        }
        return format!("{} days ago", days);
    }

    if days < 365 {
        let months = days / 30;
        if months == 1 {
            return "1 month ago".to_string();
        }
        return format!("{} months ago", months);
    }

    let years = days / 365;
    if years == 1 {
        return "1 year ago".to_string();
    }
    format!("{} years ago", years)
}

/// Format a timestamp for display, using relative time if recent or absolute time if old.
pub fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    // For timestamps older than 30 days, show the date
    if duration.num_days() > 30 {
        return timestamp.format("%Y-%m-%d %H:%M").to_string();
    }

    format_relative_time(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn format_relative_time_seconds() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now), "just now");
        assert_eq!(format_relative_time(now - Duration::seconds(30)), "just now");
        assert_eq!(format_relative_time(now - Duration::seconds(59)), "just now");
    }

    #[test]
    fn format_relative_time_minutes() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now - Duration::minutes(1)), "1 minute ago");
        assert_eq!(format_relative_time(now - Duration::minutes(2)), "2 minutes ago");
        assert_eq!(format_relative_time(now - Duration::minutes(30)), "30 minutes ago");
        assert_eq!(format_relative_time(now - Duration::minutes(59)), "59 minutes ago");
    }

    #[test]
    fn format_relative_time_hours() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now - Duration::hours(1)), "1 hour ago");
        assert_eq!(format_relative_time(now - Duration::hours(2)), "2 hours ago");
        assert_eq!(format_relative_time(now - Duration::hours(12)), "12 hours ago");
        assert_eq!(format_relative_time(now - Duration::hours(23)), "23 hours ago");
    }

    #[test]
    fn format_relative_time_days() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now - Duration::days(1)), "1 day ago");
        assert_eq!(format_relative_time(now - Duration::days(2)), "2 days ago");
        assert_eq!(format_relative_time(now - Duration::days(7)), "7 days ago");
        assert_eq!(format_relative_time(now - Duration::days(29)), "29 days ago");
    }

    #[test]
    fn format_relative_time_months() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now - Duration::days(30)), "1 month ago");
        assert_eq!(format_relative_time(now - Duration::days(60)), "2 months ago");
        assert_eq!(format_relative_time(now - Duration::days(300)), "10 months ago");
    }

    #[test]
    fn format_relative_time_years() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now - Duration::days(365)), "1 year ago");
        assert_eq!(format_relative_time(now - Duration::days(730)), "2 years ago");
    }

    #[test]
    fn format_relative_time_future() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now + Duration::hours(1)), "in the future");
    }

    #[test]
    fn format_timestamp_recent_uses_relative() {
        let now = Utc::now();
        assert_eq!(format_timestamp(now - Duration::hours(1)), "1 hour ago");
        assert_eq!(format_timestamp(now - Duration::days(7)), "7 days ago");
    }

    #[test]
    fn format_timestamp_old_uses_absolute() {
        let now = Utc::now();
        let old = now - Duration::days(60);
        let result = format_timestamp(old);
        // Should be in YYYY-MM-DD HH:MM format
        assert!(result.contains("-"), "Expected date format, got: {}", result);
        assert!(result.contains(":"), "Expected time format, got: {}", result);
    }
}
