use serde::{Deserialize, Serialize};

/// `agent_runs.status` (SCOPE.md). `waiting_approval` is the status a
/// run holds while a tool request sits unresolved in the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    WaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// A run is live while the bridge is still driving it. After a
    /// crash the reconcile re-drives only runs that are *not* live and
    /// still carry a pending or approved approval (SCOPE crash-resume).
    pub fn is_live(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::WaitingApproval)
    }
}

impl std::str::FromStr for RunStatus {
    type Err = UnknownRunStatus;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "waiting_approval" => Self::WaitingApproval,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            other => return Err(UnknownRunStatus(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRunStatus(pub String);

impl std::fmt::Display for UnknownRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown run status: {}", self.0)
    }
}
impl std::error::Error for UnknownRunStatus {}
