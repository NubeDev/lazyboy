use serde::{Deserialize, Serialize};

/// `workflows.status` (SCOPE.md "Workflows and automation"). A workflow
/// is a saved run; an automation is a workflow that is `enabled` (its
/// trigger armed). `disabled` is the saved-but-inert state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Enabled,
    Disabled,
}

impl WorkflowStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }

    /// An enabled workflow is what SCOPE.md calls an automation: its
    /// trigger is armed and the feed-watcher will fire it.
    pub fn is_automation(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

impl std::str::FromStr for WorkflowStatus {
    type Err = UnknownWorkflowStatus;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "enabled" => Self::Enabled,
            "disabled" => Self::Disabled,
            other => return Err(UnknownWorkflowStatus(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownWorkflowStatus(pub String);

impl std::fmt::Display for UnknownWorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown workflow status: {}", self.0)
    }
}
impl std::error::Error for UnknownWorkflowStatus {}
