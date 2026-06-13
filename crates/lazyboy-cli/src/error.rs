/// Errors surfaced to the operator. `Usage` prints the help banner and
/// exits 2; everything else prints the message and exits 1.
pub enum CliError {
    Usage(String),
    Store(lazyboy_store::StoreError),
    Core(lazyboy_core::CoreError),
    Bridge(lazyboy_bridge::BridgeError),
    Io(std::io::Error),
}

// The wrapped errors are all Display; route Debug through it so test
// `.unwrap()` panics carry the real message instead of a struct dump.
impl std::fmt::Debug for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(m) => write!(f, "usage: {m}"),
            Self::Store(e) => write!(f, "store: {e}"),
            Self::Core(e) => write!(f, "core: {e}"),
            Self::Bridge(e) => write!(f, "goose: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl From<lazyboy_bridge::BridgeError> for CliError {
    fn from(e: lazyboy_bridge::BridgeError) -> Self {
        Self::Bridge(e)
    }
}

impl From<lazyboy_store::StoreError> for CliError {
    fn from(e: lazyboy_store::StoreError) -> Self {
        Self::Store(e)
    }
}

impl From<lazyboy_core::CoreError> for CliError {
    fn from(e: lazyboy_core::CoreError) -> Self {
        Self::Core(e)
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
