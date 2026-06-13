use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use lazyboy_types::domain::{Identity, Space, Workspace};
use lazyboy_types::Id;

/// Host-side pointers to the single bootstrapped workspace, space, and
/// the identities that author rows. This is shell config, not domain
/// state (SCOPE.md R1): SQLite remains the source of truth; this file
/// only records *which* ids `init` minted so later commands address the
/// same space without a list-spaces query the store does not expose yet.
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub workspace: Id<Workspace>,
    pub space: Id<Space>,
    pub agent: Id<Identity>,
    pub human: Id<Identity>,
}

impl Config {
    /// The sidecar path for a given database path: `lazyboy.db` ->
    /// `lazyboy.json`, keeping config beside the data it points into.
    pub fn path_for(db_path: &str) -> PathBuf {
        Path::new(db_path).with_extension("json")
    }

    pub fn load(db_path: &str) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(Self::path_for(db_path))?;
        serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, db_path: &str) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).expect("config serializes");
        std::fs::write(Self::path_for(db_path), text)
    }
}
