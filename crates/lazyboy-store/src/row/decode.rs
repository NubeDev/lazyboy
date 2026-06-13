use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::StoreError;
use lazyboy_types::Id;

/// Parse a UUID-text column into a typed id, attributing any failure
/// to the named column.
pub(crate) fn id<T>(value: &str, column: &'static str) -> Result<Id<T>, StoreError> {
    Uuid::parse_str(value)
        .map(Id::from_uuid)
        .map_err(|e| StoreError::Decode {
            column,
            detail: e.to_string(),
        })
}

/// Parse a typed-enum column (any lazyboy-types enum implementing
/// `FromStr` with a `Display` error).
pub(crate) fn parse<T>(value: &str, column: &'static str) -> Result<T, StoreError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    value.parse().map_err(|e: T::Err| StoreError::Decode {
        column,
        detail: e.to_string(),
    })
}

/// Parse an RFC3339 timestamp column.
pub(crate) fn ts(value: &str, column: &'static str) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|e| StoreError::Decode {
        column,
        detail: e.to_string(),
    })
}
