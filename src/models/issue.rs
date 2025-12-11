use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Issue type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueType {
    Epic,
    #[default]
    Task,
    Bug,
    Request,
}

impl FromStr for IssueType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "epic" => Ok(IssueType::Epic),
            "task" => Ok(IssueType::Task),
            "bug" => Ok(IssueType::Bug),
            "request" => Ok(IssueType::Request),
            _ => Err(Error::InvalidIssueType(s.to_string())),
        }
    }
}


impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Epic => write!(f, "epic"),
            IssueType::Task => write!(f, "task"),
            IssueType::Bug => write!(f, "bug"),
            IssueType::Request => write!(f, "request"),
        }
    }
}

/// Issue state (open or closed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    #[default]
    Open,
    Closed,
}

impl std::fmt::Display for IssueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueState::Open => write!(f, "open"),
            IssueState::Closed => write!(f, "closed"),
        }
    }
}

/// Reason for closing an issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateReason {
    #[default]
    Completed,
    NotPlanned,
}

impl FromStr for StateReason {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "completed" => Ok(StateReason::Completed),
            "not_planned" | "notplanned" => Ok(StateReason::NotPlanned),
            _ => Err(Error::InvalidStateReason(s.to_string())),
        }
    }
}

impl std::fmt::Display for StateReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateReason::Completed => write!(f, "completed"),
            StateReason::NotPlanned => write!(f, "not_planned"),
        }
    }
}

/// Sort field for issue listings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortField {
    #[default]
    Updated,
    Created,
    Id,
}

/// Sort order for issue listings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

/// An issue in the tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: i64,
    pub title: String,
    pub body: Option<String>,
    #[serde(rename = "type")]
    pub issue_type: IssueType,
    pub state: IssueState,
    pub state_reason: Option<StateReason>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Data for creating a new issue
#[derive(Debug, Clone, Default)]
pub struct IssueCreate {
    pub title: String,
    pub body: Option<String>,
    pub issue_type: IssueType,
    pub labels: Vec<String>,
}

/// Filter criteria for listing issues.
///
/// Note: `Default` uses `state: None` (all states) and `limit: 30`.
/// The CLI layer should set `state: Some(IssueState::Open)` to match
/// PLAN.md's default behavior of showing only open issues.
#[derive(Debug, Clone)]
pub struct IssueFilter {
    pub state: Option<IssueState>,
    pub issue_type: Option<IssueType>,
    pub labels: Vec<String>,
    pub include_deleted: bool,
    pub sort_by: SortField,
    pub sort_order: SortOrder,
    pub limit: usize,
    pub offset: usize,
}

impl Default for IssueFilter {
    fn default() -> Self {
        Self {
            state: None,
            issue_type: None,
            labels: Vec::new(),
            include_deleted: false,
            sort_by: SortField::default(),
            sort_order: SortOrder::default(),
            limit: 30,
            offset: 0,
        }
    }
}

impl IssueFilter {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Data for updating an existing issue
#[derive(Debug, Clone, Default)]
pub struct IssueUpdate {
    pub title: Option<String>,
    pub body: Option<String>,
    pub issue_type: Option<IssueType>,
}

/// A bidirectional link between two issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLink {
    pub issue_a_id: i64,
    pub issue_b_id: i64,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_type_from_str_valid() {
        assert_eq!(IssueType::from_str("task").unwrap(), IssueType::Task);
        assert_eq!(IssueType::from_str("bug").unwrap(), IssueType::Bug);
        assert_eq!(IssueType::from_str("epic").unwrap(), IssueType::Epic);
        assert_eq!(IssueType::from_str("request").unwrap(), IssueType::Request);
    }

    #[test]
    fn issue_type_from_str_invalid() {
        assert!(IssueType::from_str("invalid").is_err());
        assert!(IssueType::from_str("").is_err());
    }

    #[test]
    fn issue_type_case_insensitive() {
        assert_eq!(IssueType::from_str("BUG").unwrap(), IssueType::Bug);
        assert_eq!(IssueType::from_str("Bug").unwrap(), IssueType::Bug);
        assert_eq!(IssueType::from_str("TASK").unwrap(), IssueType::Task);
    }

    #[test]
    fn state_reason_from_str_valid() {
        assert_eq!(
            StateReason::from_str("completed").unwrap(),
            StateReason::Completed
        );
        assert_eq!(
            StateReason::from_str("not_planned").unwrap(),
            StateReason::NotPlanned
        );
    }

    #[test]
    fn issue_filter_default_values() {
        let filter = IssueFilter::new();
        assert_eq!(filter.state, None);
        assert_eq!(filter.issue_type, None);
        assert!(filter.labels.is_empty());
        assert!(!filter.include_deleted);
        assert_eq!(filter.sort_by, SortField::Updated);
        assert_eq!(filter.sort_order, SortOrder::Desc);
        assert_eq!(filter.limit, 30);
        assert_eq!(filter.offset, 0);
    }

    #[test]
    fn issue_filter_default_and_new_are_consistent() {
        let from_default = IssueFilter::default();
        let from_new = IssueFilter::new();
        assert_eq!(from_default.limit, from_new.limit);
        assert_eq!(from_default.limit, 30);
    }

    #[test]
    fn issue_serializes_to_json() {
        let issue = Issue {
            id: 42,
            title: "Test issue".to_string(),
            body: Some("Body text".to_string()),
            issue_type: IssueType::Bug,
            state: IssueState::Open,
            state_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
            deleted_at: None,
        };

        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"title\":\"Test issue\""));
        assert!(json.contains("\"type\":\"bug\""));
        assert!(json.contains("\"state\":\"open\""));
    }

    #[test]
    fn issue_link_serializes_to_json() {
        let link = IssueLink {
            issue_a_id: 1,
            issue_b_id: 2,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("\"issue_a_id\":1"));
        assert!(json.contains("\"issue_b_id\":2"));
    }
}
