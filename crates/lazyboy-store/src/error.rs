/// Errors crossing the store boundary. `Decode` wraps the typed-enum
/// parse failures from lazyboy-types so a corrupt status column is a
/// real error, not a silent default.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error("row not found: {0}")]
    NotFound(String),

    #[error("decode column {column}: {detail}")]
    Decode {
        column: &'static str,
        detail: String,
    },
}

impl StoreError {
    /// True when this is a SQL uniqueness conflict (a duplicate slug, a
    /// re-used external ref). Callers turn it into a client-facing 400
    /// without reaching into `sqlx` internals themselves.
    pub fn is_unique_violation(&self) -> bool {
        matches!(
            self,
            StoreError::Sqlx(sqlx::Error::Database(db)) if db.is_unique_violation()
        )
    }
}
