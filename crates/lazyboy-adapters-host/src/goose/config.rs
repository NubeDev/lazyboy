//! The Lazyboy-owned goose provider configuration: which provider and
//! model to run, plus the per-provider API keys.
//!
//! Lazyboy owns this rather than goose's own `config.yaml` because the
//! supervisor launches `goose serve` with the selection injected into its
//! environment (`GOOSE_PROVIDER`, `GOOSE_MODEL`, the provider's
//! `*_API_KEY`); owning the source of truth is what makes a UI provider
//! switch take effect on the next launch, headlessly, with no keyring
//! prompt. Two files under the config dir:
//!   - `goose.json`    — provider + model (non-secret)
//!   - `goose-secrets.json` — keys by env-var name, mode 0600
//! The secrets file is never returned to a caller; the store only reports
//! which providers have a key set (SCOPE.md R5: the UI sees a flag, not
//! the secret).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use lazyboy_bridge::BridgeError;

use super::catalog;

/// The non-secret selection persisted to `goose.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    /// Goose provider id (e.g. `anthropic`); `None` until first set.
    pub provider: Option<String>,
    /// Model id within the provider; `None` falls back to goose's default.
    pub model: Option<String>,
}

/// Keys stored by their goose env-var name (e.g. `ANTHROPIC_API_KEY`), so
/// the supervisor injects each verbatim.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Secrets {
    #[serde(default)]
    keys: BTreeMap<String, String>,
}

/// Reads and writes the goose provider configuration under a config dir.
/// Cheap to construct; holds only the resolved directory.
#[derive(Clone)]
pub struct GooseConfigStore {
    dir: PathBuf,
}

impl GooseConfigStore {
    /// Resolve the store at `$XDG_CONFIG_HOME/lazyboy` (or `$HOME/.config/
    /// lazyboy`). The directory is created on first write, not here, so
    /// constructing the store is side-effect free.
    pub fn discover() -> Result<Self, BridgeError> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .ok_or_else(|| BridgeError::Config("neither XDG_CONFIG_HOME nor HOME is set".into()))?;
        Ok(Self {
            dir: base.join("lazyboy"),
        })
    }

    fn selection_path(&self) -> PathBuf {
        self.dir.join("goose.json")
    }

    fn secrets_path(&self) -> PathBuf {
        self.dir.join("goose-secrets.json")
    }

    /// The persisted provider/model selection, or the default (both
    /// `None`) when nothing has been saved yet.
    pub fn selection(&self) -> Result<Selection, BridgeError> {
        match std::fs::read(self.selection_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| BridgeError::Config(format!("goose.json parse: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Selection::default()),
            Err(e) => Err(BridgeError::Config(format!("read goose.json: {e}"))),
        }
    }

    /// Whether a key is stored for the given env var.
    pub fn has_key(&self, key_env: &str) -> Result<bool, BridgeError> {
        Ok(self.secrets()?.keys.contains_key(key_env))
    }

    fn secrets(&self) -> Result<Secrets, BridgeError> {
        match std::fs::read(self.secrets_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| BridgeError::Config(format!("goose-secrets.json parse: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Secrets::default()),
            Err(e) => Err(BridgeError::Config(format!("read goose-secrets.json: {e}"))),
        }
    }

    /// Persist a provider/model selection and, when supplied, the API key
    /// for that provider. An unknown provider id is rejected (the catalog
    /// is the boundary). A `None` key leaves any stored key untouched, so
    /// switching model without re-entering the key works; an empty-string
    /// key clears it. Returns the saved selection.
    pub fn save(
        &self,
        provider: &str,
        model: Option<&str>,
        key: Option<&str>,
    ) -> Result<Selection, BridgeError> {
        let spec = catalog::find(provider)
            .ok_or_else(|| BridgeError::Config(format!("unknown provider: {provider}")))?;

        std::fs::create_dir_all(&self.dir)
            .map_err(|e| BridgeError::Config(format!("create config dir: {e}")))?;

        if let Some(key) = key {
            let mut secrets = self.secrets()?;
            if key.is_empty() {
                secrets.keys.remove(spec.key_env);
            } else {
                secrets.keys.insert(spec.key_env.to_owned(), key.to_owned());
            }
            self.write_secrets(&secrets)?;
        }

        let selection = Selection {
            provider: Some(provider.to_owned()),
            model: model.filter(|m| !m.is_empty()).map(str::to_owned),
        };
        let bytes = serde_json::to_vec_pretty(&selection)
            .map_err(|e| BridgeError::Config(format!("serialize goose.json: {e}")))?;
        std::fs::write(self.selection_path(), bytes)
            .map_err(|e| BridgeError::Config(format!("write goose.json: {e}")))?;
        Ok(selection)
    }

    /// The environment a `goose serve` launch needs for the current
    /// selection: `GOOSE_PROVIDER`, `GOOSE_MODEL` (when set), and the
    /// provider's `*_API_KEY` (when a key is stored). Empty when no
    /// provider has been selected, so the supervisor can decline to launch
    /// rather than start a provider-less goose that fails every turn.
    pub fn launch_env(&self) -> Result<Vec<(String, String)>, BridgeError> {
        let selection = self.selection()?;
        let Some(provider) = selection.provider.as_deref() else {
            return Ok(Vec::new());
        };
        let Some(spec) = catalog::find(provider) else {
            return Ok(Vec::new());
        };
        let mut env = vec![("GOOSE_PROVIDER".to_owned(), provider.to_owned())];
        if let Some(model) = selection.model {
            env.push(("GOOSE_MODEL".to_owned(), model));
        }
        if let Some(key) = self.secrets()?.keys.get(spec.key_env) {
            env.push((spec.key_env.to_owned(), key.clone()));
        }
        Ok(env)
    }

    /// Write the secrets file with owner-only permissions (0600 on unix)
    /// so the keys are not world-readable. The mode is set before the
    /// content is written by creating the file with the restricted mode.
    fn write_secrets(&self, secrets: &Secrets) -> Result<(), BridgeError> {
        let bytes = serde_json::to_vec_pretty(secrets)
            .map_err(|e| BridgeError::Config(format!("serialize secrets: {e}")))?;
        let path = self.secrets_path();
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)
                .map_err(|e| BridgeError::Config(format!("open secrets 0600: {e}")))?;
            f.write_all(&bytes)
                .map_err(|e| BridgeError::Config(format!("write secrets: {e}")))?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&path, bytes)
                .map_err(|e| BridgeError::Config(format!("write secrets: {e}")))?;
        }
        Ok(())
    }
}
