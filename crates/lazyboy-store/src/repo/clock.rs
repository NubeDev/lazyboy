use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Render a timestamp into the RFC3339 text the timeline columns
/// store. Infallible for well-known: the format always succeeds.
pub(crate) fn fmt(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .expect("rfc3339 is total over OffsetDateTime")
}

/// The current wall-clock instant in UTC, used as the default `ts`
/// when a caller does not supply one.
pub(crate) fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
