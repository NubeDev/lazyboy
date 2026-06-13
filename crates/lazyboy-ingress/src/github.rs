use serde_json::Value;

use crate::{NormalizeError, NormalizedEvent};

/// Normalize a GitHub webhook payload. MVP targets the issue/PR comment
/// and issue/PR opened shapes — the highest-signal events (SCOPE.md).
///
/// `external_id`: the comment's node/id when present, else the
/// issue/PR's, prefixed by kind so a comment and the issue it hangs off
/// never collide. `body`: the comment or issue body, prefixed with the
/// repo and action for a readable timeline line.
pub fn normalize(payload: &Value) -> Result<NormalizedEvent, NormalizeError> {
    let action = payload.get("action").and_then(Value::as_str);
    let repo = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown-repo");

    if let Some(comment) = payload.get("comment") {
        let id = comment
            .get("id")
            .map(value_id)
            .ok_or(NormalizeError::MissingField("comment.id"))?;
        let body = comment
            .get("body")
            .and_then(Value::as_str)
            .ok_or(NormalizeError::MissingField("comment.body"))?;
        return Ok(NormalizedEvent {
            external_id: format!("comment:{id}"),
            kind: "comment".to_owned(),
            body: format!("[{repo}] comment: {body}"),
        });
    }

    let issue = payload
        .get("issue")
        .or_else(|| payload.get("pull_request"))
        .ok_or(NormalizeError::MissingField("issue|pull_request"))?;
    let id = issue
        .get("id")
        .map(value_id)
        .ok_or(NormalizeError::MissingField("issue.id"))?;
    let title = issue
        .get("title")
        .and_then(Value::as_str)
        .ok_or(NormalizeError::MissingField("issue.title"))?;
    let action = action.unwrap_or("event");
    Ok(NormalizedEvent {
        external_id: format!("issue:{id}"),
        kind: "issue".to_owned(),
        body: format!("[{repo}] {action}: {title}"),
    })
}

/// GitHub numeric ids arrive as JSON numbers; render them as text for a
/// stable string `external_id` without losing precision.
fn value_id(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn comment_payload_keys_on_comment_id() {
        let payload = json!({
            "action": "created",
            "repository": {"full_name": "acme/web"},
            "comment": {"id": 42, "body": "ship it"}
        });
        let event = normalize(&payload).unwrap();
        assert_eq!(event.external_id, "comment:42");
        assert_eq!(event.kind, "comment");
        assert!(event.body.contains("acme/web"));
        assert!(event.body.contains("ship it"));
    }

    #[test]
    fn issue_payload_keys_on_issue_id() {
        let payload = json!({
            "action": "opened",
            "repository": {"full_name": "acme/web"},
            "issue": {"id": 7, "title": "broken nav"}
        });
        let event = normalize(&payload).unwrap();
        assert_eq!(event.external_id, "issue:7");
        assert_eq!(event.kind, "issue");
        assert!(event.body.contains("opened"));
    }

    #[test]
    fn comment_without_id_is_missing_field() {
        let payload = json!({"comment": {"body": "no id"}});
        assert_eq!(
            normalize(&payload),
            Err(NormalizeError::MissingField("comment.id"))
        );
    }
}
