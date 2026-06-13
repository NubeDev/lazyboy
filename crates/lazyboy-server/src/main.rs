//! Thin entrypoint for the browser-shell backend. Reads the same env as
//! the CLI for the store and goose seam, plus a listen address and the
//! single-tenant bearer, then delegates to `lazyboy_server::serve`.

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let db = std::env::var("LAZYBOY_DB").unwrap_or_else(|_| "lazyboy.db".to_owned());
    let db_url = format!("sqlite://{db}");
    let goose_url =
        std::env::var("GOOSE_URL").unwrap_or_else(|_| "http://127.0.0.1:3284".to_owned());
    let addr: SocketAddr = std::env::var("LAZYBOY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:7878".to_owned())
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("error: invalid LAZYBOY_ADDR: {e}");
            std::process::exit(2);
        });
    // Unset disables auth (dev); set requires `Authorization: Bearer`.
    let token = std::env::var("LAZYBOY_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());

    if let Err(e) = lazyboy_server::serve(addr, &db_url, goose_url, token).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
