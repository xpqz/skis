use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Not a skis repository (or any parent up to /). Run 'skis init' to create one.")]
    NotARepository,

    #[error("Already initialized")]
    AlreadyInitialized,

    #[error("Issue #{0} not found")]
    IssueNotFound(i64),

    #[error("Comment #{0} not found")]
    CommentNotFound(i64),

    #[error("Label '{0}' not found. Create it with: skis label create {0}")]
    LabelNotFound(String),

    #[error("Issue #{0} is already {1}")]
    InvalidStateTransition(i64, String),

    #[error("Invalid color '{0}': must be 6 hex characters (e.g., ff0000)")]
    InvalidColor(String),

    #[error("Invalid issue type '{0}': must be epic, task, bug, or request")]
    InvalidIssueType(String),

    #[error("Invalid state reason '{0}': must be completed or not_planned")]
    InvalidStateReason(String),

    #[error("Cannot link issue to itself")]
    SelfLink,

    #[error("Link already exists between issues #{0} and #{1}")]
    DuplicateLink(i64, i64),

    #[error("{0}: not yet implemented")]
    NotImplemented(String),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_not_a_repository_message() {
        let err = Error::NotARepository;
        assert_eq!(
            err.to_string(),
            "Not a skis repository (or any parent up to /). Run 'skis init' to create one."
        );
    }

    #[test]
    fn error_label_not_found_suggests_create() {
        let err = Error::LabelNotFound("bug".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Label 'bug' not found"));
        assert!(msg.contains("skis label create bug"));
    }

    #[test]
    fn error_invalid_state_transition_message() {
        let err = Error::InvalidStateTransition(42, "closed".to_string());
        assert_eq!(err.to_string(), "Issue #42 is already closed");
    }

    #[test]
    fn error_invalid_color_message() {
        let err = Error::InvalidColor("gggggg".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid color 'gggggg'"));
        assert!(msg.contains("6 hex characters"));
    }

    #[test]
    fn error_issue_not_found_message() {
        let err = Error::IssueNotFound(999);
        assert_eq!(err.to_string(), "Issue #999 not found");
    }

    #[test]
    fn error_invalid_issue_type_message() {
        let err = Error::InvalidIssueType("foo".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid issue type 'foo'"));
        assert!(msg.contains("epic, task, bug, or request"));
    }

    #[test]
    fn error_invalid_state_reason_message() {
        let err = Error::InvalidStateReason("bar".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid state reason 'bar'"));
        assert!(msg.contains("completed or not_planned"));
    }
}
