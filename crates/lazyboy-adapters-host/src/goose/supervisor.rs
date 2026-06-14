//! Owns the `goose serve` child process so a UI provider switch can take
//! effect: goose reads its provider, model, and key from the environment
//! at launch, so applying a new selection means relaunching goose with
//! the new `launch_env`. This is the only place Lazyboy spawns goose
//! (`std::process` / `tokio::process` is confined to this host-only crate
//! per SCOPE.md R1); the server/CLI shells drive it, mobile-safe crates
//! never reach it.

use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use lazyboy_bridge::BridgeError;

use super::config::GooseConfigStore;

/// How goose is reached and launched: the vendored binary, the bind
/// address `goose serve` should listen on, and the config store the
/// launch env comes from.
#[derive(Clone)]
pub struct GooseSupervisor {
    inner: Arc<Inner>,
}

struct Inner {
    binary: String,
    host: String,
    port: u16,
    store: GooseConfigStore,
    child: Mutex<Option<Child>>,
}

impl GooseSupervisor {
    /// `binary` is the path to the vendored goose (e.g. `bin/goose`);
    /// `addr` is where `goose serve` should listen, parsed from the
    /// configured goose url's host:port.
    pub fn new(binary: String, addr: SocketAddr, store: GooseConfigStore) -> Self {
        Self {
            inner: Arc::new(Inner {
                binary,
                host: addr.ip().to_string(),
                port: addr.port(),
                store,
                child: Mutex::new(None),
            }),
        }
    }

    /// Whether the supervised child is currently alive. A child that has
    /// exited (or was never started) reports `false`; this does not probe
    /// the ACP port, only the process.
    pub async fn running(&self) -> bool {
        let mut guard = self.inner.child.lock().await;
        match guard.as_mut() {
            None => false,
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => false,
                Ok(None) => true,
                Err(_) => false,
            },
        }
    }

    /// Kill any running child and spawn a fresh `goose serve` with the
    /// current selection's env. Declines (without spawning) when no
    /// provider is configured yet, so the caller can report "configure a
    /// provider" rather than launch a goose that fails every turn. Waits
    /// until the ACP port accepts a connection before returning, so a
    /// caller that connects right after sees a ready server.
    pub async fn restart(&self) -> Result<(), BridgeError> {
        let env = self.inner.store.launch_env()?;
        if env.is_empty() {
            return Err(BridgeError::Config(
                "no goose provider configured; set one before starting goose".into(),
            ));
        }

        self.stop().await;

        let mut cmd = Command::new(&self.inner.binary);
        cmd.arg("serve")
            .arg("--host")
            .arg(&self.inner.host)
            .arg("--port")
            .arg(self.inner.port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = cmd
            .spawn()
            .map_err(|e| BridgeError::Config(format!("spawn goose serve: {e}")))?;
        *self.inner.child.lock().await = Some(child);

        self.wait_ready().await
    }

    /// Kill the running child if any, ignoring an already-exited one.
    pub async fn stop(&self) {
        if let Some(mut child) = self.inner.child.lock().await.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }

    /// Poll the ACP port until it accepts a TCP connection or the budget
    /// elapses. goose serve binds shortly after spawn; a connect success
    /// is the readiness signal the bridge's own connect then relies on.
    async fn wait_ready(&self) -> Result<(), BridgeError> {
        let addr = format!("{}:{}", self.inner.host, self.inner.port);
        for _ in 0..50 {
            if tokio::net::TcpStream::connect(&addr).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(BridgeError::Config(format!(
            "goose serve did not start listening on {addr} within 5s"
        )))
    }
}
