use std::str::FromStr;

use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use lazyboy_store::repo;
use lazyboy_types::domain::{Reminder, ReminderStatus, Space, Task, TaskState};
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;

/// Parse an RFC3339 timestamp argument the agent supplies for a reminder
/// or calendar time, mirroring the create-reminder route's parsing so the
/// agent and the HTTP API accept the same format.
fn parse_time(args: &Value, field: &str) -> Result<OffsetDateTime, ApiError> {
    let raw = args
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::BadRequest(format!("{field} is required (RFC3339 UTC)")))?;
    OffsetDateTime::parse(raw, &Rfc3339)
        .map_err(|_| ApiError::BadRequest(format!("{field} must be RFC3339, e.g. 2026-06-14T13:00:00Z")))
}

/// The tools the rented agent may call against the space it is scoped to.
/// Read tools carry `readOnlyHint: true` so goose's permission gate lets
/// them through without an approval prompt; `create_task` is an internal
/// lazyboy mutation (not an outside-world action under SCOPE.md R6), so
/// it is annotated non-destructive. Genuine outside-world tools (shell,
/// git, http) live in other goose extensions and stay gated unchanged.
pub fn definitions() -> Vec<Value> {
    let empty = json!({ "type": "object", "properties": {}, "additionalProperties": false });
    vec![
        json!({
            "name": "space_overview",
            "description": "Summarise the current space: its open tasks, the most recent timeline messages, and any pending reminders. Call this first when asked for an overview or status of the space.",
            "inputSchema": empty,
            "annotations": { "title": "Space overview", "readOnlyHint": true },
        }),
        json!({
            "name": "list_tasks",
            "description": "List every task in the current space with its id and state (open, running, blocked_on_approval, done, cancelled). Use the id with set_task_state to change a task.",
            "inputSchema": empty,
            "annotations": { "title": "List tasks", "readOnlyHint": true },
        }),
        json!({
            "name": "create_task",
            "description": "Open a new task in the current space from a short imperative title (e.g. 'Ship the pricing page'). Use this when the user asks to add, create, or track a task.",
            "inputSchema": json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short imperative task title." }
                },
                "required": ["title"],
                "additionalProperties": false,
            }),
            "annotations": {
                "title": "Create task",
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": false,
                "openWorldHint": false,
            },
        }),
        json!({
            "name": "set_task_state",
            "description": "Change a task's state by id (get ids from list_tasks). Use this to close/complete a task (state 'done'), cancel it ('cancelled'), or reopen it ('open'). Never create a new task to represent closing one.",
            "inputSchema": json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The task id from list_tasks." },
                    "state": {
                        "type": "string",
                        "enum": ["open", "done", "cancelled"],
                        "description": "Target state: 'done' to complete/close, 'cancelled' to drop, 'open' to reopen.",
                    },
                },
                "required": ["task_id", "state"],
                "additionalProperties": false,
            }),
            "annotations": {
                "title": "Set task state",
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": true,
                "openWorldHint": false,
            },
        }),
        json!({
            "name": "set_reminder",
            "description": "Create a reminder in the current space. Use this when the user wants to be reminded of something at a time. `due_at` is an absolute RFC3339 UTC timestamp computed from the current time.",
            "inputSchema": json!({
                "type": "object",
                "properties": {
                    "body": { "type": "string", "description": "What to be reminded about." },
                    "due_at": { "type": "string", "description": "When to fire, RFC3339 UTC (e.g. 2026-06-14T13:00:00Z)." },
                },
                "required": ["body", "due_at"],
                "additionalProperties": false,
            }),
            "annotations": { "title": "Set reminder", "readOnlyHint": false, "destructiveHint": false, "openWorldHint": false },
        }),
        json!({
            "name": "list_reminders",
            "description": "List the pending reminders in the current space, with their ids and due times.",
            "inputSchema": empty,
            "annotations": { "title": "List reminders", "readOnlyHint": true },
        }),
        json!({
            "name": "dismiss_reminder",
            "description": "Dismiss a reminder by id (get ids from list_reminders). Use this when the user no longer needs it.",
            "inputSchema": json!({
                "type": "object",
                "properties": { "reminder_id": { "type": "string", "description": "The reminder id from list_reminders." } },
                "required": ["reminder_id"],
                "additionalProperties": false,
            }),
            "annotations": { "title": "Dismiss reminder", "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false },
        }),
        json!({
            "name": "create_calendar_event",
            "description": "Add a calendar event to the current space. Use this for meetings, appointments, or anything scheduled at a time. Times are RFC3339 UTC computed from the current time.",
            "inputSchema": json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short event title." },
                    "starts_at": { "type": "string", "description": "Start time, RFC3339 UTC." },
                    "ends_at": { "type": "string", "description": "Optional end time, RFC3339 UTC." },
                },
                "required": ["title", "starts_at"],
                "additionalProperties": false,
            }),
            "annotations": { "title": "Create calendar event", "readOnlyHint": false, "destructiveHint": false, "openWorldHint": false },
        }),
        json!({
            "name": "list_calendar",
            "description": "List the calendar events in the current space, with their ids and times.",
            "inputSchema": empty,
            "annotations": { "title": "List calendar", "readOnlyHint": true },
        }),
    ]
}

/// Execute one tool call, returning the text the agent receives back.
/// Every tool resolves the space from the transport header (see
/// [`super::space`]), never a tool argument.
pub async fn call(
    state: &AppState,
    space_id: Id<Space>,
    name: &str,
    args: &Value,
) -> Result<String, ApiError> {
    match name {
        "space_overview" => space_overview(state, space_id).await,
        "list_tasks" => list_tasks(state, space_id).await,
        "create_task" => create_task(state, space_id, args).await,
        "set_task_state" => set_task_state(state, space_id, args).await,
        "set_reminder" => set_reminder(state, space_id, args).await,
        "list_reminders" => list_reminders(state, space_id).await,
        "dismiss_reminder" => dismiss_reminder(state, space_id, args).await,
        "create_calendar_event" => create_calendar_event(state, space_id, args).await,
        "list_calendar" => list_calendar(state, space_id).await,
        other => Err(ApiError::BadRequest(format!("unknown tool: {other}"))),
    }
}

async fn list_tasks(state: &AppState, space_id: Id<Space>) -> Result<String, ApiError> {
    let tasks = repo::task::list(state.store(), space_id).await?;
    if tasks.is_empty() {
        return Ok("No tasks in this space yet.".to_owned());
    }
    let lines: Vec<String> = tasks
        .iter()
        .map(|t| format!("- [{}] {} (id: {})", t.state.as_str(), t.title, t.id))
        .collect();
    Ok(lines.join("\n"))
}

async fn set_task_state(
    state: &AppState,
    space_id: Id<Space>,
    args: &Value,
) -> Result<String, ApiError> {
    let task_id = args
        .get("task_id")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_value::<Id<Task>>(Value::String(s.to_owned())).ok())
        .ok_or_else(|| ApiError::BadRequest("set_task_state needs a valid task_id".to_owned()))?;
    let target = args
        .get("state")
        .and_then(Value::as_str)
        .and_then(|s| TaskState::from_str(s).ok())
        .ok_or_else(|| {
            ApiError::BadRequest("set_task_state needs state open|done|cancelled".to_owned())
        })?;

    // Confirm the task belongs to this space before mutating it — the
    // space binding is the trust boundary, so a task id from another space
    // must not resolve here.
    let tasks = repo::task::list(state.store(), space_id).await?;
    let task = tasks
        .into_iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| ApiError::NotFound(format!("no task {task_id} in this space")))?;

    repo::task::set_state(state.store(), task_id, target).await?;
    Ok(format!(
        "Task '{}' is now {}.",
        task.title,
        target.as_str()
    ))
}

async fn create_task(
    state: &AppState,
    space_id: Id<Space>,
    args: &Value,
) -> Result<String, ApiError> {
    let title = args
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ApiError::BadRequest("create_task needs a non-empty title".to_owned()))?;
    let id = repo::task::create(state.store(), space_id, title, None).await?;
    Ok(format!("Created task '{title}' (id {id})."))
}

async fn set_reminder(
    state: &AppState,
    space_id: Id<Space>,
    args: &Value,
) -> Result<String, ApiError> {
    let body = args
        .get("body")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .ok_or_else(|| ApiError::BadRequest("set_reminder needs a non-empty body".to_owned()))?;
    let due_at = parse_time(args, "due_at")?;
    let id = repo::reminder::create(
        state.store(),
        repo::reminder::NewReminder {
            space_id,
            task_id: None,
            due_at,
            body,
        },
    )
    .await?;
    Ok(format!("Reminder set: '{body}' (id {id})."))
}

async fn list_reminders(state: &AppState, space_id: Id<Space>) -> Result<String, ApiError> {
    let rows = repo::reminder::list(state.store(), space_id).await?;
    let pending: Vec<_> = rows
        .iter()
        .filter(|r| r.status == ReminderStatus::Pending)
        .collect();
    if pending.is_empty() {
        return Ok("No pending reminders.".to_owned());
    }
    let lines: Vec<String> = pending
        .iter()
        .map(|r| format!("- {} (due {}, id: {})", r.body, r.due_at, r.id))
        .collect();
    Ok(lines.join("\n"))
}

async fn dismiss_reminder(
    state: &AppState,
    space_id: Id<Space>,
    args: &Value,
) -> Result<String, ApiError> {
    let reminder_id = args
        .get("reminder_id")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_value::<Id<Reminder>>(Value::String(s.to_owned())).ok())
        .ok_or_else(|| {
            ApiError::BadRequest("dismiss_reminder needs a valid reminder_id".to_owned())
        })?;
    // The space binding is the trust boundary: a reminder id from another
    // space must not resolve here.
    let reminder = repo::reminder::list(state.store(), space_id)
        .await?
        .into_iter()
        .find(|r| r.id == reminder_id)
        .ok_or_else(|| ApiError::NotFound(format!("no reminder {reminder_id} in this space")))?;
    let changed =
        repo::reminder::set_status(state.store(), reminder_id, ReminderStatus::Dismissed).await?;
    Ok(if changed {
        format!("Dismissed reminder '{}'.", reminder.body)
    } else {
        format!("Reminder '{}' was already dismissed.", reminder.body)
    })
}

async fn create_calendar_event(
    state: &AppState,
    space_id: Id<Space>,
    args: &Value,
) -> Result<String, ApiError> {
    let title = args
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            ApiError::BadRequest("create_calendar_event needs a non-empty title".to_owned())
        })?;
    let starts_at = parse_time(args, "starts_at")?;
    let ends_at = match args.get("ends_at").and_then(Value::as_str) {
        Some(_) => Some(parse_time(args, "ends_at")?),
        None => None,
    };
    let id = repo::calendar::upsert(
        state.store(),
        repo::calendar::NewCalendarEvent {
            space_id,
            source: "local",
            external_ref: None,
            title,
            starts_at,
            ends_at,
            meta_json: None,
        },
    )
    .await?;
    Ok(format!("Calendar event '{title}' added (id {id})."))
}

async fn list_calendar(state: &AppState, space_id: Id<Space>) -> Result<String, ApiError> {
    let rows = repo::calendar::list(state.store(), space_id, repo::calendar::Window::default()).await?;
    if rows.is_empty() {
        return Ok("No calendar events.".to_owned());
    }
    let lines: Vec<String> = rows
        .iter()
        .map(|e| {
            let ends = e.ends_at.map(|t| format!(" – {t}")).unwrap_or_default();
            format!("- {} ({}{}, id: {})", e.title, e.starts_at, ends, e.id)
        })
        .collect();
    Ok(lines.join("\n"))
}

async fn space_overview(state: &AppState, space_id: Id<Space>) -> Result<String, ApiError> {
    let store = state.store();
    let tasks = repo::task::list(store, space_id).await?;
    let messages = repo::message::list(store, space_id).await?;
    let reminders = repo::reminder::list(store, space_id).await?;
    let calendar = repo::calendar::list(store, space_id, repo::calendar::Window::default()).await?;

    let mut out = String::new();
    out.push_str(&format!("Tasks ({}):\n", tasks.len()));
    if tasks.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for t in &tasks {
            out.push_str(&format!(
                "  - [{}] {} (id: {})\n",
                t.state.as_str(),
                t.title,
                t.id
            ));
        }
    }

    // The timeline is append-ordered; the tail is the freshest activity.
    let recent: Vec<_> = messages.iter().rev().take(10).collect();
    out.push_str(&format!("\nRecent messages ({} shown):\n", recent.len()));
    if recent.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for m in recent.iter().rev() {
            let body = m.body.replace('\n', " ");
            let snippet: String = body.chars().take(120).collect();
            out.push_str(&format!("  - {}: {}\n", m.kind.as_str(), snippet));
        }
    }

    let pending: Vec<_> = reminders
        .iter()
        .filter(|r| r.status == ReminderStatus::Pending)
        .collect();
    out.push_str(&format!("\nPending reminders ({}):\n", pending.len()));
    for r in &pending {
        out.push_str(&format!("  - {} (due {})\n", r.body, r.due_at));
    }

    out.push_str(&format!("\nCalendar events ({}):\n", calendar.len()));
    for e in &calendar {
        out.push_str(&format!("  - {} ({})\n", e.title, e.starts_at));
    }

    Ok(out)
}
