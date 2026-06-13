use serde::{Deserialize, Serialize};

/// `approvals.status` (SCOPE.md). This row is Lazyboy's trust layer:
/// it is written the moment Goose requests a tool and survives a crash
/// independent of the runtime, so the approval stays in the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }

    /// Pending and approved approvals are the ones the crash-resume
    /// reconcile must re-drive against Goose; a denied one is settled.
    pub fn needs_resume(self) -> bool {
        matches!(self, Self::Pending | Self::Approved)
    }
}

impl std::str::FromStr for ApprovalStatus {
    type Err = UnknownApprovalStatus;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "denied" => Self::Denied,
            other => return Err(UnknownApprovalStatus(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownApprovalStatus(pub String);

impl std::fmt::Display for UnknownApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown approval status: {}", self.0)
    }
}
impl std::error::Error for UnknownApprovalStatus {}
