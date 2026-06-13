use serde::Deserialize;
use serde_json::Value;

use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// The explicit ingress routing stored in `integrations.config_json`
/// (SCOPE.md): MVP routes an event to a space by an operator-declared
/// binding, never by content inference. Shape:
/// `{"bindings":[{"repo":"owner/x","space_id":"..."}]}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Bindings {
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

/// One binding: a subscription key (a GitHub repo, a Gmail label, a
/// thread, a Slack channel) and the space it routes into. The key fields
/// are optional so one shape covers every provider; a binding matches an
/// event when each present key equals the event's corresponding key. A
/// binding with no keys is a catch-all for its integration.
#[derive(Debug, Clone, Deserialize)]
pub struct Binding {
    pub space_id: Id<Space>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub thread: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
}

/// Resolve which space a raw payload routes into, from the integration's
/// stored bindings. The provider-specific routing key is read straight
/// from the payload (the repo's `full_name` for GitHub, a Gmail label
/// id, etc.); the first binding whose declared keys all match wins, so
/// place catch-alls last. Returns `None` when no binding matches — the
/// caller decides whether that is a 4xx or a drop.
pub fn resolve_space(bindings: &Bindings, payload: &Value) -> Option<Id<Space>> {
    let repo = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(Value::as_str);
    let labels = payload
        .get("labelIds")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    let thread = payload.get("threadId").and_then(Value::as_str);
    let channel = payload.get("channel").and_then(Value::as_str);

    bindings.bindings.iter().find_map(|b| {
        let repo_ok = match b.repo.as_deref() {
            Some(want) => Some(want) == repo,
            None => true,
        };
        let label_ok = match b.label.as_deref() {
            Some(want) => labels.contains(&want),
            None => true,
        };
        let thread_ok = match b.thread.as_deref() {
            Some(want) => Some(want) == thread,
            None => true,
        };
        let channel_ok = match b.channel.as_deref() {
            Some(want) => Some(want) == channel,
            None => true,
        };
        if repo_ok && label_ok && thread_ok && channel_ok {
            Some(b.space_id)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repo_binding_routes_matching_event() {
        let space = Id::<Space>::new();
        let bindings = Bindings {
            bindings: vec![Binding {
                space_id: space,
                repo: Some("acme/web".to_owned()),
                label: None,
                thread: None,
                channel: None,
            }],
        };
        let payload = json!({"repository": {"full_name": "acme/web"}});
        assert_eq!(resolve_space(&bindings, &payload), Some(space));

        let other = json!({"repository": {"full_name": "acme/api"}});
        assert_eq!(resolve_space(&bindings, &other), None);
    }

    #[test]
    fn keyless_binding_is_catch_all() {
        let space = Id::<Space>::new();
        let bindings = Bindings {
            bindings: vec![Binding {
                space_id: space,
                repo: None,
                label: None,
                thread: None,
                channel: None,
            }],
        };
        assert_eq!(resolve_space(&bindings, &json!({})), Some(space));
    }
}
