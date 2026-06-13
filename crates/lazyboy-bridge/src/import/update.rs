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
            record_event(store, ctx, seq, "agent_message", text).await?;
            append(store, ctx, MessageKind::Agent, text, None).await?;
            Ok(Imported::Recorded)
        }

        Update::ToolResult {
            tool_name,
            output_json,
        } => {
            record_event(store, ctx, seq, "tool_result", output_json).await?;
            let body = format!("{tool_name}: {output_json}");
            append(store, ctx, MessageKind::ToolResult, &body, None).await?;
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
