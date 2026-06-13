use serde::{Deserialize, Serialize};

/// `workflows.trigger_kind` (SCOPE.md "Workflows and automation"). What
/// starts a saved run: a feed event arriving in a bound space, or a
/// schedule firing. The live arming of either trigger is a host-side
/// daemon concern; this enum only names which one a workflow carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    Feed,
    Schedule,
}

impl TriggerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feed => "feed",
            Self::Schedule => "schedule",
        }
    }
}

impl std::str::FromStr for TriggerKind {
    type Err = UnknownTriggerKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "feed" => Self::Feed,
            "schedule" => Self::Schedule,
            other => return Err(UnknownTriggerKind(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTriggerKind(pub String);

impl std::fmt::Display for UnknownTriggerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown trigger kind: {}", self.0)
    }
}
impl std::error::Error for UnknownTriggerKind {}
