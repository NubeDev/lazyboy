use lazyboy_types::domain::ApprovalStatus;

/// The human decision the bridge sends back for a `PermissionRequest`.
/// Mirrors the resolvable approval statuses; a pending approval has no
/// decision to send yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    /// Map a resolved approval status to a decision, or `None` if the
    /// status is not a terminal human decision.
    pub fn from_status(status: ApprovalStatus) -> Option<Self> {
        match status {
            ApprovalStatus::Approved => Some(Self::Allow),
            ApprovalStatus::Denied => Some(Self::Deny),
            ApprovalStatus::Pending => None,
        }
    }
}
