use serde::{Deserialize, Serialize};

/// `tasks.state` (SCOPE.md). `blocked_on_approval` is the state a task
/// sits in while its run waits on a pending `approvals` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Open,
    Running,
    BlockedOnApproval,
    Done,
    Cancelled,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Running => "running",
            Self::BlockedOnApproval => "blocked_on_approval",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for TaskState {
    type Err = UnknownTaskState;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "open" => Self::Open,
            "running" => Self::Running,
            "blocked_on_approval" => Self::BlockedOnApproval,
            "done" => Self::Done,
            "cancelled" => Self::Cancelled,
            other => return Err(UnknownTaskState(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTaskState(pub String);

impl std::fmt::Display for UnknownTaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown task state: {}", self.0)
    }
}
impl std::error::Error for UnknownTaskState {}
