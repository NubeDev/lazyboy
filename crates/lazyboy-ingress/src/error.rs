use lazyboy_types::domain::Provider;

/// Why a payload could not be normalized. The normalizers are strict:
/// a payload missing the field that yields the dedup `external_id` is a
/// hard error, never a silently-defaulted id, so dedup stays sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizeError {
    /// A field required to derive the `external_id` or body is absent.
    MissingField(&'static str),
    /// The provider is registered but has no MVP normalizer yet.
    Unsupported(Provider),
}

impl std::fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "ingress payload missing field: {field}"),
            Self::Unsupported(provider) => {
                write!(
                    f,
                    "no ingress normalizer for provider: {}",
                    provider.as_str()
                )
            }
        }
    }
}
impl std::error::Error for NormalizeError {}
