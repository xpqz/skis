use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A comment on an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub issue_id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_serializes_to_json() {
        let comment = Comment {
            id: 1,
            issue_id: 42,
            body: "This is a comment".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&comment).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"issue_id\":42"));
        assert!(json.contains("\"body\":\"This is a comment\""));
    }
}
