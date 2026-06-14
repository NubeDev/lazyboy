use lazyboy_store::{repo, Store};
use lazyboy_types::domain::MessageKind;

use super::{ImportContext, Imported};
use crate::acp::Update;
use crate::BridgeError;

/// Apply one update to the store. `seq` is the run-local order the
/// driver assigns; every update is recorded as an `agent_run_events`
/// row, and the user-visible ones also append a timeline message.
///
/// `PermissionRequested` is the load-bearing case: it writes the
/// durable `approvals` row (SCOPE.md R6) plus a `tool_request` message
/// referencing it, and returns `AwaitingApproval` so the driver pauses.
pub async fn import_update(
    store: &Store,
    ctx: &ImportContext,
    seq: i64,
    update: &Update,
) -> Result<Imported, BridgeError> {
    match update {
        Update::AgentMessage { text } => {
            // Record the chunk in the audit log (one event row per chunk),
            // but hand the text back for the driver to coalesce into a
            // single timeline message — see `Imported::AgentChunk`.
            record_event(store, ctx, seq, "agent_message", text).await?;
            Ok(Imported::AgentChunk { text: text.clone() })
        }

        Update::ToolResult {
            tool_name,
            output_json,
        } => {
            // Tool results stay in the durable event log (audit) and still
            // surface as artifacts when they carry an output locator, but
            // they are no longer dumped into the timeline as raw
            // `{name}: {json}` rows: that filled the channel with tool
            // plumbing the agent already narrates in its own message.
            let _ = tool_name;
            record_event(store, ctx, seq, "tool_result", output_json).await?;
            import_artifact(store, ctx, output_json).await?;
            Ok(Imported::Recorded)
        }

        Update::PermissionRequested(req) => {
            record_event(
                store,
                ctx,
                seq,
                "permission_requested",
                &req.tool.input_json,
            )
            .await?;
            let approval_id = repo::approval::request(
                store,
                repo::approval::NewApproval {
                    space_id: ctx.space_id,
                    agent_run_id: ctx.agent_run_id,
                    goose_session_id: &ctx.goose_session_id,
                    tool_name: &req.tool.name,
                    tool_input_json: &req.tool.input_json,
                },
            )
            .await?;
            let body = format!("requests {}: {}", req.tool.name, req.tool.input_json);
            append(
                store,
                ctx,
                MessageKind::ToolRequest,
                &body,
                Some(approval_id.to_string()),
            )
            .await?;
            Ok(Imported::AwaitingApproval {
                approval_id,
                request_id: req.request_id.clone(),
            })
        }

        Update::TurnEnded { stopped } => {
            record_event(store, ctx, seq, "turn_ended", &stopped.to_string()).await?;
            Ok(Imported::TurnEnded {
                succeeded: *stopped,
            })
        }
    }
}

/// Promote a tool result to a durable artifact when it carries an
/// explicit output locator (a written file, a URL, a PR, a patch). The
/// `tool_result` message already records what ran; the artifact row and
/// its `artifact_ref` message are the run's *output*, the thing the
/// space keeps after the run ends (SCOPE.md "results land back as
/// artifacts"). A result with no locator stays a plain tool result.
async fn import_artifact(
    store: &Store,
    ctx: &ImportContext,
    output_json: &str,
) -> Result<(), BridgeError> {
    let Some(detected) = super::artifact::detect(output_json) else {
        return Ok(());
    };
    let artifact_id = repo::artifact::create(
        store,
        repo::artifact::NewArtifact {
            space_id: ctx.space_id,
            agent_run_id: ctx.agent_run_id,
            kind: detected.kind,
            uri: &detected.uri,
            meta_json: Some(output_json),
        },
    )
    .await?;
    let body = format!("{}: {}", detected.kind, detected.uri);
    append(
        store,
        ctx,
        MessageKind::ArtifactRef,
        &body,
        Some(artifact_id.to_string()),
    )
    .await?;
    Ok(())
}

async fn record_event(
    store: &Store,
    ctx: &ImportContext,
    seq: i64,
    kind: &str,
    payload: &str,
) -> Result<(), BridgeError> {
    repo::run::append_event(
        store,
        repo::run::NewRunEvent {
            run_id: ctx.agent_run_id,
            seq,
            kind,
            payload_json: payload,
        },
    )
    .await?;
    Ok(())
}

/// Append the coalesced text of a run of streamed agent chunks as one
/// timeline message. The driver calls this to flush its buffer once the
/// contiguous chunk run ends (a non-chunk update or turn end), so the
/// timeline shows one agent message per turn while the event log keeps
/// the per-chunk audit rows.
pub async fn append_agent_message(
    store: &Store,
    ctx: &ImportContext,
    text: &str,
) -> Result<(), BridgeError> {
    append(store, ctx, MessageKind::Agent, text, None).await
}

async fn append(
    store: &Store,
    ctx: &ImportContext,
    kind: MessageKind,
    body: &str,
    ref_id: Option<String>,
) -> Result<(), BridgeError> {
    repo::message::append(
        store,
        repo::message::NewMessage {
            space_id: ctx.space_id,
            author: ctx.agent_identity,
            kind,
            body,
            ref_id,
        },
    )
    .await?;
    Ok(())
}
