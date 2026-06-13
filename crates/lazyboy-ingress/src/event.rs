/// A provider event reduced to what the store's ingest verb needs: a
/// stable `external_id` (the dedup key, SCOPE.md), a coarse `kind`, and
/// the human-readable `body` the timeline message carries. The raw
/// payload is kept by the caller for the `ingress_events` audit row;
/// this struct is the transport-free projection of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedEvent {
    pub external_id: String,
    pub kind: String,
    pub body: String,
}
