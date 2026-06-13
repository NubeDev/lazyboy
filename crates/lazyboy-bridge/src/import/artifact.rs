/// A produced artifact recognised in a tool result, before it is
/// written to the store. `kind` matches the SCOPE.md artifact kinds
/// (`file` | `pr` | `url` | `patch`); `uri` locates it.
pub(crate) struct DetectedArtifact {
    pub kind: &'static str,
    pub uri: String,
}

/// Decide whether a tool result produced something worth recording as an
/// `artifact`, and what kind.
///
/// The heuristic is deliberately conservative and structural rather than
/// guessing from free text: a tool result is parsed as JSON and only an
/// explicit, named locator promotes it to an artifact. Free-form tool
/// chatter that merely *mentions* a path or URL is left as an ordinary
/// `tool_result` message — a false artifact row is worse than a missing
/// one, because the artifact view is meant to be the run's durable
/// outputs, not a scrape of its logs.
///
/// Recognised locators, in priority order:
/// - `html_url`/`url`/`uri` whose value is an `http(s)` link. A GitHub
///   `/pull/` link is a `pr`; any other link is a `url`.
/// - `path`/`file`/`filename` naming a written file -> `file`.
/// - `patch`/`diff` carrying a unified diff -> `patch` (the URI is the
///   field name, since the patch body lives in `meta_json`).
pub(crate) fn detect(output_json: &str) -> Option<DetectedArtifact> {
    let value: serde_json::Value = serde_json::from_str(output_json).ok()?;
    let obj = locator_object(&value)?;

    for key in ["html_url", "url", "uri"] {
        if let Some(link) = obj.get(key).and_then(|v| v.as_str()) {
            if link.starts_with("http://") || link.starts_with("https://") {
                let kind = if is_pull_request(link) { "pr" } else { "url" };
                return Some(DetectedArtifact {
                    kind,
                    uri: link.to_owned(),
                });
            }
        }
    }

    for key in ["path", "file", "filename"] {
        if let Some(path) = obj.get(key).and_then(|v| v.as_str()) {
            if !path.is_empty() {
                return Some(DetectedArtifact {
                    kind: "file",
                    uri: path.to_owned(),
                });
            }
        }
    }

    for key in ["patch", "diff"] {
        if obj.get(key).and_then(|v| v.as_str()).is_some() {
            return Some(DetectedArtifact {
                kind: "patch",
                uri: key.to_owned(),
            });
        }
    }

    None
}

/// The object whose fields we inspect. A tool result may wrap its
/// payload (`{"output": {...}}`); look one level in so a wrapped locator
/// is still found, but no deeper — unbounded recursion would reopen the
/// false-positive risk the structural rule exists to avoid.
fn locator_object(
    value: &serde_json::Value,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
    if let Some(obj) = value.as_object() {
        for wrapper in ["output", "result", "data"] {
            if let Some(inner) = obj.get(wrapper).and_then(|v| v.as_object()) {
                return Some(inner);
            }
        }
        return Some(obj);
    }
    None
}

fn is_pull_request(link: &str) -> bool {
    link.contains("github.com") && link.contains("/pull/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_field_is_a_url_artifact() {
        let a = detect(r#"{"url":"https://example.com/x"}"#).unwrap();
        assert_eq!(a.kind, "url");
        assert_eq!(a.uri, "https://example.com/x");
    }

    #[test]
    fn github_pull_link_is_a_pr() {
        let a = detect(r#"{"html_url":"https://github.com/o/r/pull/3"}"#).unwrap();
        assert_eq!(a.kind, "pr");
    }

    #[test]
    fn path_field_is_a_file() {
        let a = detect(r#"{"path":"src/main.rs"}"#).unwrap();
        assert_eq!(a.kind, "file");
        assert_eq!(a.uri, "src/main.rs");
    }

    #[test]
    fn wrapped_locator_is_found_one_level_in() {
        let a = detect(r#"{"output":{"path":"a.txt"}}"#).unwrap();
        assert_eq!(a.kind, "file");
    }

    #[test]
    fn plain_text_output_is_not_an_artifact() {
        assert!(detect(r#"{"stdout":"see https://example.com in the logs"}"#).is_none());
        assert!(detect("not json at all").is_none());
    }

    #[test]
    fn patch_body_is_a_patch_artifact() {
        let a = detect(r#"{"diff":"--- a\n+++ b\n"}"#).unwrap();
        assert_eq!(a.kind, "patch");
        assert_eq!(a.uri, "diff");
    }
}
