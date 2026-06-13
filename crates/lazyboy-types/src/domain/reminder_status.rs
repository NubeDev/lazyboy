use serde::{Deserialize, Serialize};

/// `reminders.status` (SCOPE.md). A reminder is `pending` until its
/// `due_at` passes and the firing pass moves it to `fired`; a human can
/// `dismiss` it at any point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    Pending,
    Fired,
    Dismissed,
}

impl ReminderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Fired => "fired",
            Self::Dismissed => "dismissed",
        }
    }
}

impl std::str::FromStr for ReminderStatus {
    type Err = UnknownReminderStatus;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pending" => Self::Pending,
            "fired" => Self::Fired,
            "dismissed" => Self::Dismissed,
            other => return Err(UnknownReminderStatus(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReminderStatus(pub String);

impl std::fmt::Display for UnknownReminderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown reminder status: {}", self.0)
    }
}
impl std::error::Error for UnknownReminderStatus {}
