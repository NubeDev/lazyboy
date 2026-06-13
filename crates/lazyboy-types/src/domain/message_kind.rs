use serde::{Deserialize, Serialize};

/// `messages.kind` (SCOPE.md). A typed message carries a `ref_id` to
/// the approvals/artifacts/decisions/ingress row it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Human,
    Agent,
    System,
    ToolRequest,
    ToolResult,
    ArtifactRef,
    DecisionRef,
    Ingress,
}

impl MessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::System => "system",
            Self::ToolRequest => "tool_request",
            Self::ToolResult => "tool_result",
            Self::ArtifactRef => "artifact_ref",
            Self::DecisionRef => "decision_ref",
            Self::Ingress => "ingress",
        }
    }
}

impl std::str::FromStr for MessageKind {
    type Err = UnknownMessageKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "human" => Self::Human,
            "agent" => Self::Agent,
            "system" => Self::System,
            "tool_request" => Self::ToolRequest,
            "tool_result" => Self::ToolResult,
            "artifact_ref" => Self::ArtifactRef,
            "decision_ref" => Self::DecisionRef,
            "ingress" => Self::Ingress,
            other => return Err(UnknownMessageKind(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownMessageKind(pub String);

impl std::fmt::Display for UnknownMessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown message kind: {}", self.0)
    }
}
impl std::error::Error for UnknownMessageKind {}
