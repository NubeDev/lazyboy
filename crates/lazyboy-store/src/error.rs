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
