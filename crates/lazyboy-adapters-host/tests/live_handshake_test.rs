//! Exercises `GooseServeClient` against a real `goose serve`. Verifies
//! the transport contract that needs no model provider: the WebSocket
//! carries the connection id, `initialize` reports `loadSession`, and
//! `session/new`'s id arrives over the WS (not the `202` POST body) —
//! the exact shape `DOCS/GOOSE-ACP.md` records. A live model turn needs
//! `goose configure`; that is out of scope here.
//!
//! Skips (does not fail) when `bin/goose` is absent, so CI without the
//! pinned binary stays green.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use lazyboy_adapters_host::GooseServeClient;

struct Serve(Child);

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn goose_bin() -> Option<std::path::PathBuf> {
    // tests run with CWD at the crate root; the binary is workspace-level.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../bin/goose")
        .canonicalize()
        .ok()?;
    path.exists().then_some(path)
}

#[tokio::test]
async fn handshake_and_new_session_over_ws() {
    let Some(bin) = goose_bin() else {
        eprintln!("skipping: bin/goose not installed (run `make install-goose`)");
        return;
    };

    let port = 38291;
    let child = Command::new(&bin)
        .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn goose serve");
    let _serve = Serve(child);

    let base = format!("http://127.0.0.1:{port}");
    let client = connect_with_retry(&base).await;

    // session/new acknowledges with 202; the id must come back over the
    // WS. If the transport waited on the POST body this would hang/empty.
    let session = lazyboy_bridge::GooseClient::new_session(&client, "test-space")
        .await
        .expect("new_session");
    assert!(
        !session.0.is_empty(),
        "session id arrived over the websocket"
    );
}

async fn connect_with_retry(base: &str) -> GooseServeClient {
    for _ in 0..50 {
        if let Ok(c) = GooseServeClient::connect(base).await {
            return c;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("goose serve did not become ready at {base}");
}
