use serde::{Deserialize, Serialize};

/// `integrations.provider` (SCOPE.md). The external system a feed
/// ingresses from; MVP lands GitHub and Gmail first (highest signal),
/// with Slack and Google Calendar following.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Github,
    Gmail,
    Slack,
    Gcal,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gmail => "gmail",
            Self::Slack => "slack",
            Self::Gcal => "gcal",
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = UnknownProvider;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "github" => Self::Github,
            "gmail" => Self::Gmail,
            "slack" => Self::Slack,
            "gcal" => Self::Gcal,
            other => return Err(UnknownProvider(other.to_owned())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownProvider(pub String);

impl std::fmt::Display for UnknownProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown provider: {}", self.0)
    }
}
impl std::error::Error for UnknownProvider {}
