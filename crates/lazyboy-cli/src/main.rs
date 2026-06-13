//! The local shell for Lazyboy's step-1 demo (SCOPE.md build order):
//! one space, local, driving a live `goose serve`, surviving restart
//! through the crash-resume reconcile. Deliberately thin — it parses
//! args and delegates to `lazyboy_cli::commands`, which wire the store,
//! the host transport, and the engine together. Richer clients (the
//! React UI) talk to the same core through `RpcClient`, not this.

use std::str::FromStr;

use lazyboy_cli::{commands, CliError, Config};
use lazyboy_store::Store;
use lazyboy_types::domain::{Approval, ApprovalStatus};
use lazyboy_types::Id;

const USAGE: &str = "\
lazyboy — local agent runner (step-1 shell)

usage:
  lazyboy init [space-name]            bootstrap the workspace and space
  lazyboy run <prompt>                 start a run; pause on the first approval
  lazyboy approve <approval-id>        approve a pending tool, resume the run
  lazyboy deny <approval-id>           deny a pending tool, resume the run
  lazyboy pending                      list approvals awaiting a decision
  lazyboy timeline                     print the space timeline

env:
  LAZYBOY_DB    sqlite path           (default: lazyboy.db)
  GOOSE_URL     goose serve base url  (default: http://127.0.0.1:3284)";

#[tokio::main]
async fn main() {
    match dispatch().await {
        Ok(()) => {}
        Err(CliError::Usage(msg)) => {
            eprintln!("{msg}\n\n{USAGE}");
            std::process::exit(2);
        }
        Err(CliError::Store(e)) => fail(format!("store: {e}")),
        Err(CliError::Core(e)) => fail(format!("core: {e}")),
        Err(CliError::Bridge(e)) => fail(format!("goose: {e}")),
        Err(CliError::Io(e)) => fail(format!("io: {e}")),
    }
}

fn fail(msg: String) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn db_path() -> String {
    std::env::var("LAZYBOY_DB").unwrap_or_else(|_| "lazyboy.db".to_owned())
}

fn goose_url() -> String {
    std::env::var("GOOSE_URL").unwrap_or_else(|_| "http://127.0.0.1:3284".to_owned())
}

async fn dispatch() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args
        .first()
        .ok_or_else(|| CliError::Usage("no command given".to_owned()))?;
    let db = db_path();
    let store = Store::connect(&format!("sqlite://{db}")).await?;

    match cmd.as_str() {
        "init" => {
            let space = args.get(1).map(String::as_str).unwrap_or("home");
            commands::init(&store, &db, space).await
        }
        "run" => {
            let prompt = rest(&args).ok_or_else(|| CliError::Usage("run needs a prompt".into()))?;
            commands::run(&store, &load_config(&db)?, &goose_url(), &prompt).await
        }
        "approve" => decide_cmd(&store, &db, &args, ApprovalStatus::Approved).await,
        "deny" => decide_cmd(&store, &db, &args, ApprovalStatus::Denied).await,
        "pending" => commands::pending(&store, &load_config(&db)?).await,
        "timeline" => commands::timeline(&store, &load_config(&db)?).await,
        other => Err(CliError::Usage(format!("unknown command: {other}"))),
    }
}

async fn decide_cmd(
    store: &Store,
    db: &str,
    args: &[String],
    status: ApprovalStatus,
) -> Result<(), CliError> {
    let raw = args
        .get(1)
        .ok_or_else(|| CliError::Usage("need an approval id".into()))?;
    let uuid = uuid::Uuid::from_str(raw)
        .map_err(|_| CliError::Usage(format!("not a valid approval id: {raw}")))?;
    let approval_id = Id::<Approval>::from_uuid(uuid);
    commands::decide(store, &load_config(db)?, &goose_url(), approval_id, status).await
}

fn load_config(db: &str) -> Result<Config, CliError> {
    Config::load(db)
        .map_err(|_| CliError::Usage("not initialized; run `lazyboy init` first".to_owned()))
}

/// Join everything after the subcommand into one prompt string.
fn rest(args: &[String]) -> Option<String> {
    if args.len() < 2 {
        return None;
    }
    Some(args[1..].join(" "))
}
