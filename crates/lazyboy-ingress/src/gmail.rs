use serde_json::Value;

use crate::{NormalizeError, NormalizedEvent};

/// Normalize a Gmail message resource (the `users.messages.get` shape).
///
/// `external_id`: the message `id` — stable across redelivery, the dedup
/// key (SCOPE.md). `body`: the subject and from header with the snippet,
/// for a readable timeline line. Headers live in `payload.headers` as a
/// `[{name, value}]` list; we pull Subject and From by name.
pub fn normalize(payload: &Value) -> Result<NormalizedEvent, NormalizeError> {
    let id = payload
        .get("id")
        .and_then(Value::as_str)
        .ok_or(NormalizeError::MissingField("id"))?;

    let subject = header(payload, "Subject").unwrap_or("(no subject)");
    let from = header(payload, "From").unwrap_or("(unknown sender)");
    let snippet = payload.get("snippet").and_then(Value::as_str).unwrap_or("");

    Ok(NormalizedEvent {
        external_id: id.to_owned(),
        kind: "email".to_owned(),
        body: format!("from {from} — {subject}: {snippet}"),
    })
}

fn header<'a>(payload: &'a Value, name: &str) -> Option<&'a str> {
    payload
        .get("payload")
        .and_then(|p| p.get("headers"))
        .and_then(Value::as_array)?
        .iter()
        .find(|h| h.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|h| h.get("value"))
        .and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_keys_on_id_and_reads_headers() {
        let payload = json!({
            "id": "18ab",
            "snippet": "can we meet?",
            "payload": {"headers": [
                {"name": "Subject", "value": "Lunch"},
                {"name": "From", "value": "ada@acme.io"}
            ]}
        });
        let event = normalize(&payload).unwrap();
        assert_eq!(event.external_id, "18ab");
        assert_eq!(event.kind, "email");
        assert!(event.body.contains("ada@acme.io"));
        assert!(event.body.contains("Lunch"));
        assert!(event.body.contains("can we meet?"));
    }

    #[test]
    fn message_without_id_is_missing_field() {
        let payload = json!({"snippet": "x"});
        assert_eq!(normalize(&payload), Err(NormalizeError::MissingField("id")));
    }

    #[test]
    fn missing_headers_fall_back_to_placeholders() {
        let event = normalize(&json!({"id": "z"})).unwrap();
        assert!(event.body.contains("(no subject)"));
    }
}
