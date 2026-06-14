use lazyboy_adapters_host::GooseServeClient;
use lazyboy_core::{Engine, RunOutcome};
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{AgentRun, Approval, ApprovalStatus};
use lazyboy_types::Id;

use crate::config::Config;
use crate::CliError;

/// Bootstrap the single workspace, space, and identities, recording their
/// ids in the sidecar. Idempotent only in the sense that it refuses to
/// clobber an existing config; re-init means a fresh db.
pub async fn init(store: &Store, db_path: &str, space_name: &str) -> Result<(), CliError> {
    if Config::path_for(db_path).exists() {
        return Err(CliError::Usage(format!(
            "already initialized ({} exists)",
            Config::path_for(db_path).display()
        )));
    }
    let workspace = repo::bootstrap::create_workspace(store, "default").await?;
    let space = repo::bootstrap::create_space(store, workspace, space_name, space_name).await?;
    let agent = repo::bootstrap::create_identity(
        store,
        workspace,
        repo::bootstrap::NewIdentity {
            kind: "agent",
            display_name: "Goose",
            external_ref: None,
        },
    )
    .await?;
    let human = repo::bootstrap::create_identity(
        store,
        workspace,
        repo::bootstrap::NewIdentity {
            kind: "human",
            display_name: "operator",
            external_ref: None,
        },
    )
    .await?;
    Config {
        workspace,
        space,
        agent,
        human,
    }
    .save(db_path)?;
    println!("initialized space '{space_name}' ({space})");
    Ok(())
}

/// Connect to goose, recover any in-flight approvals, then start a run
/// from the prompt and report where it paused. This is the step-1 loop:
/// the reconcile call is what makes a mid-approval crash survivable.
pub async fn run(
    store: &Store,
    cfg: &Config,
    goose_url: &str,
    prompt: &str,
) -> Result<(), CliError> {
    let client = GooseServeClient::connect(goose_url).await?;
    let engine = Engine::new(store.clone(), client, cfg.agent);

    let recovered = engine.reconcile().await?;
    if !recovered.is_empty() {
        println!(
            "recovered {} in-flight approval(s) on startup",
            recovered.len()
        );
    }

    let before = repo::message::list(store, cfg.space).await?.len();
    let started = engine.start_chat(cfg.space, prompt).await?;
    print_new_messages(store, cfg, before).await?;
    report_outcome(&started.outcome, store, cfg).await?;
    Ok(())
}

/// Apply a decision to a pending approval. Reconnects and reconciles
/// first so the decision works even in a fresh process after a crash
/// (the request id is rebuilt by re-driving the loaded session).
pub async fn decide(
    store: &Store,
    cfg: &Config,
    goose_url: &str,
    approval_id: Id<Approval>,
    status: ApprovalStatus,
) -> Result<(), CliError> {
    let client = GooseServeClient::connect(goose_url).await?;
    let engine = Engine::new(store.clone(), client, cfg.agent);
    engine.reconcile().await?;

    let before = repo::message::list(store, cfg.space).await?.len();
    match engine
        .resolve_approval(approval_id, status, cfg.human)
        .await?
    {
        None => {
            println!("approval {approval_id} was already resolved; no-op");
            return Ok(());
        }
        Some(outcome) => {
            print_new_messages(store, cfg, before).await?;
            report_outcome(&outcome, store, cfg).await?;
        }
    }
    Ok(())
}

/// Print where a run paused and, if it is parked, the approvals waiting
/// on a human. Shared by `run` and `decide` so both report uniformly.
async fn report_outcome(outcome: &RunOutcome, store: &Store, cfg: &Config) -> Result<(), CliError> {
    match outcome {
        RunOutcome::AwaitingApproval => {
            println!("paused on an approval:");
            print_pending(store, cfg).await?;
        }
        RunOutcome::Ended { succeeded } => println!(
            "run ended ({})",
            if *succeeded { "succeeded" } else { "failed" }
        ),
    }
    Ok(())
}

/// Print the timeline messages appended since `before` — the rows this
/// action produced — so `run` and `decide` show the agent's output
/// inline instead of forcing a separate `timeline` call.
async fn print_new_messages(store: &Store, cfg: &Config, before: usize) -> Result<(), CliError> {
    for m in repo::message::list(store, cfg.space)
        .await?
        .into_iter()
        .skip(before)
    {
        println!("[{}] {}", m.kind.as_str(), m.body);
    }
    Ok(())
}

/// Cancel a run: mark it cancelled and deny any approval parked on it.
/// Deliberately does not connect to goose — cancel is pure store work
/// (the durable rows are the truth, SCOPE.md R1), so it must succeed
/// even when goose is down, which is exactly when a stuck run is
/// cancelled. Mirrors `Engine::cancel_run` without a transport.
pub async fn cancel(store: &Store, cfg: &Config, run_id: Id<AgentRun>) -> Result<(), CliError> {
    let run = repo::run::get(store, run_id).await?;
    repo::approval::deny_pending_for_run(store, run_id, cfg.human).await?;
    repo::run::set_status(store, run_id, lazyboy_types::domain::RunStatus::Cancelled).await?;
    // A chat turn has no task; only a task-backed run cancels its task.
    if let Some(task_id) = run.task_id {
        repo::task::set_state(store, task_id, lazyboy_types::domain::TaskState::Cancelled).await?;
    }
    println!("cancelled run {run_id}");
    Ok(())
}

/// Retry a run: start a fresh run for the same task with the same
/// prompt, and report where it paused.
pub async fn retry(
    store: &Store,
    cfg: &Config,
    goose_url: &str,
    run_id: Id<AgentRun>,
) -> Result<(), CliError> {
    let client = GooseServeClient::connect(goose_url).await?;
    let engine = Engine::new(store.clone(), client, cfg.agent);
    engine.reconcile().await?;

    let before = repo::message::list(store, cfg.space).await?.len();
    let started = engine.retry_run(run_id).await?;
    print_new_messages(store, cfg, before).await?;
    report_outcome(&started.outcome, store, cfg).await?;
    Ok(())
}

pub async fn pending(store: &Store, cfg: &Config) -> Result<(), CliError> {
    print_pending(store, cfg).await
}

pub async fn timeline(store: &Store, cfg: &Config) -> Result<(), CliError> {
    for m in repo::message::list(store, cfg.space).await? {
        println!("[{}] {}", m.kind.as_str(), m.body);
    }
    Ok(())
}

async fn print_pending(store: &Store, cfg: &Config) -> Result<(), CliError> {
    let rows = repo::approval::list_pending(store, cfg.space).await?;
    if rows.is_empty() {
        println!("no pending approvals");
    }
    for a in rows {
        println!("{}  {}  {}", a.id, a.tool_name, a.tool_input_json);
    }
    Ok(())
}
