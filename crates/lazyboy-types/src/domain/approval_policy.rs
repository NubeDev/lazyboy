use serde::{Deserialize, Serialize};

/// `workflows.approval_policy` (SCOPE.md "Workflows and automation",
/// R6). The user's per-workflow choice of how outside-world steps are
/// gated:
///
/// - `require_approval` (default): every step parks a pending
///   `approvals` row, exactly like an interactive run.
/// - `auto_approve`: the sole sanctioned R6 exception. The `approvals`
///   row is still written first for audit, then auto-resolves instead
///   of parking, so the run proceeds unattended. Chosen per workflow by
///   a human, never a global gate-off switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    RequireApproval,
    AutoApprove,
}

impl ApprovalPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RequireApproval => "require_approval",
            Self::AutoApprove => "auto_approve",
        }
    }
}

impl std::str::FromStr for ApprovalPolicy {
    type Err = UnknownApprovalPolicy;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "require_approval" => Self::RequireApproval,
            "auto_approve" => Self::AutoApprove,
            other => return Err(UnknownApprovalPolicy(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownApprovalPolicy(pub String);

impl std::fmt::Display for UnknownApprovalPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown approval policy: {}", self.0)
    }
}
impl std::error::Error for UnknownApprovalPolicy {}
