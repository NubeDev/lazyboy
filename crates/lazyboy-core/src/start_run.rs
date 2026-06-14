use lazyboy_bridge::GooseClient;
use lazyboy_store::repo;
use lazyboy_types::domain::{AgentRun, MessageKind, Space, Task};
use lazyboy_types::Id;

use crate::drive::DriveStop;
use crate::engine::Engine;
use crate::CoreError;

/// How many recent timeline messages a chat turn carries to goose as
/// context so a follow-up ("close it", "do that") resolves against what
/// just happened. Each goose session is fresh, so without this the agent
/// has no memory of the previous turn.
const CHAT_CONTEXT_MESSAGES: usize = 12;

/// Orientation prepended to every prompt sent to goose (not stored, not
/// shown in the timeline — the human message keeps the raw prompt). It
/// frames the agent as a space assistant whose source of truth is the
/// lazyboy tools, so a capable model does not wander to the filesystem or
/// goose's own developer/todo tools when asked about "this space". This
/// is steering through the API, not a goose fork (SCOPE.md R3).
const SPACE_ASSISTANT_ORIENTATION: &str = "\
You are the assistant inside a Lazyboy space — a focused team chat channel for one initiative. \
\"This space\" always means this Lazyboy space and the tasks, messages, and reminders in it — \
never the filesystem, the project source tree, or a code directory.

To see or change anything in the space, use the lazyboy tools as your source of truth: \
`space_overview` for an overview or status, `list_tasks` to list tasks (with their ids), \
`create_task` to open a task, and `set_task_state` to close, complete, cancel, or reopen a task \
by id. To close or complete a task, call `set_task_state` with that task's id — never create a \
new task to represent closing one. When the user refers to \"it\", \"that task\", or \"the last \
one\", resolve it from the recent conversation and the task list, do not invent a new task.

For reminders and calendar, use the matching tools: `set_reminder` (text plus an absolute \
`due_at`), `list_reminders`, and `dismiss_reminder` by id; `create_calendar_event` (a title plus \
`starts_at`, optional `ends_at`) and `list_calendar`. Times are RFC3339 UTC (e.g. \
2026-06-14T13:00:00Z); compute them from the current time given below — \"in 2 hours\", \"at 1pm \
today\", \"tomorrow morning\" all become an absolute UTC timestamp you pass in.

Keep the task title itself short and imperative (e.g. \"Pick up Lenny at 1pm\") — do not put the \
user's whole sentence in the title. Do not run shell commands, read or edit files, or keep a todo \
list unless the user explicitly asks you to work with code or files. Reply briefly and \
conversationally, like a message in a chat channel.";

/// Frame a workflow/task-backed prompt: just the orientation. Kept
/// separate from what is stored/displayed so a retry frames the same raw
/// prompt identically.
pub(crate) fn frame_prompt(prompt: &str) -> String {
    format!("{SPACE_ASSISTANT_ORIENTATION}\n\n{prompt}")
}

/// The run a `start_run`/`start_chat` kicked off and where it paused.
pub struct StartedRun {
    /// `None` for a chat turn (no task is created); `Some` for a
    /// task-backed run such as a workflow.
    pub task_id: Option<Id<Task>>,
    pub run_id: Id<AgentRun>,
    pub outcome: RunOutcome,
}

/// Where the initial drive landed: blocked on an approval, or finished
/// the turn without needing one.
#[derive(Debug, PartialEq, Eq)]
pub enum RunOutcome {
    AwaitingApproval,
    Ended { succeeded: bool },
}

impl<G: GooseClient> Engine<G> {
    /// A chat turn in a space: the agent acts on the space through its
    /// lazyboy tools and replies in the timeline. Unlike `start_run` this
    /// creates **no task** — a message is not a task — and it carries the
    /// recent conversation so follow-ups have context.
    pub async fn start_chat(
        &self,
        space_id: Id<Space>,
        prompt: &str,
    ) -> Result<StartedRun, CoreError> {
        // Capture context before appending this turn's own message so the
        // framing is prior conversation, then the current prompt.
        let context = self.recent_context(space_id).await?;
        let run_id = repo::run::create(&self.store, space_id, None).await?;
        self.append_human(space_id, prompt).await?;

        // The agent needs "now" to turn "in 2 hours" or "at 1pm" into the
        // absolute RFC3339 timestamp a reminder/calendar tool wants.
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let clock = format!("Current time (UTC): {now}");
        let framed = match context {
            Some(ctx) => {
                format!("{SPACE_ASSISTANT_ORIENTATION}\n\n{clock}\n\n{ctx}\n\nUser: {prompt}")
            }
            None => format!("{SPACE_ASSISTANT_ORIENTATION}\n\n{clock}\n\nUser: {prompt}"),
        };
        let outcome = self.open_and_drive(space_id, run_id, prompt, &framed).await?;
        Ok(StartedRun {
            task_id: None,
            run_id,
            outcome,
        })
    }

    /// A task-backed run: open a task and run, then drive goose. Used for
    /// workflows and any flow where the run *is* a unit of work, not a
    /// conversation turn.
    pub async fn start_run(
        &self,
        space_id: Id<Space>,
        title: &str,
        prompt: &str,
    ) -> Result<StartedRun, CoreError> {
        let task_id = repo::task::create(&self.store, space_id, title, None).await?;
        let run_id = repo::run::create(&self.store, space_id, Some(task_id)).await?;
        repo::task::attach_run(&self.store, task_id, run_id).await?;
        self.append_human(space_id, prompt).await?;

        let outcome = self
            .open_and_drive(space_id, run_id, prompt, &frame_prompt(prompt))
            .await?;
        Ok(StartedRun {
            task_id: Some(task_id),
            run_id,
            outcome,
        })
    }

    /// Open a goose session for the run, store the raw prompt as its first
    /// durable event (so a retry re-sends it), send `framed` to goose, and
    /// drive to the first approval gate or the end of the turn.
    async fn open_and_drive(
        &self,
        space_id: Id<Space>,
        run_id: Id<AgentRun>,
        raw_prompt: &str,
        framed: &str,
    ) -> Result<RunOutcome, CoreError> {
        let session = self.goose.new_session(&space_id.to_string()).await?;
        repo::run::set_session(&self.store, run_id, session.as_str()).await?;
        self.append_prompt_event(run_id, raw_prompt).await?;
        self.goose.prompt(&session, framed).await?;
        self.drive_outcome(run_id).await
    }

    async fn drive_outcome(&self, run_id: Id<AgentRun>) -> Result<RunOutcome, CoreError> {
        Ok(match self.drive(run_id).await? {
            DriveStop::Approval => RunOutcome::AwaitingApproval,
            DriveStop::Ended { succeeded } => RunOutcome::Ended { succeeded },
            DriveStop::Drained => RunOutcome::Ended { succeeded: false },
        })
    }

    /// Persist the prompt as the run's first event so a retry re-sends the
    /// same prompt from the durable stream (SCOPE.md R1), never an
    /// in-memory copy. The seq is drawn from the same per-run counter
    /// drive() uses, so it occupies slot 1 and imported updates number
    /// from 2.
    async fn append_prompt_event(
        &self,
        run_id: Id<AgentRun>,
        prompt: &str,
    ) -> Result<(), CoreError> {
        repo::run::append_event(
            &self.store,
            repo::run::NewRunEvent {
                run_id,
                seq: self.next_seq(run_id),
                kind: "prompt",
                payload_json: prompt,
            },
        )
        .await?;
        Ok(())
    }

    /// Record the prompt as a human timeline message so the sender's own
    /// turn is visible, not just the agent's reply. Authored by the human
    /// principal when one exists; an automation-sourced run with no human
    /// falls back to the agent principal so the row still has a valid
    /// author.
    async fn append_human(&self, space_id: Id<Space>, prompt: &str) -> Result<(), CoreError> {
        let author = repo::identity::find_by_kind(&self.store, "human")
            .await?
            .unwrap_or(self.agent_identity);
        repo::message::append(
            &self.store,
            repo::message::NewMessage {
                space_id,
                author,
                kind: MessageKind::Human,
                body: prompt,
                ref_id: None,
            },
        )
        .await?;
        Ok(())
    }

    /// The recent human/agent exchange in a space, oldest-first, as plain
    /// `Role: text` lines for the model's context. Tool-plumbing and
    /// system rows are skipped; only what a person would read as the
    /// conversation is included.
    async fn recent_context(&self, space_id: Id<Space>) -> Result<Option<String>, CoreError> {
        let messages = repo::message::list(&self.store, space_id).await?;
        let lines: Vec<String> = messages
            .iter()
            .filter_map(|m| match m.kind {
                MessageKind::Human => Some(format!("User: {}", m.body)),
                MessageKind::Agent => Some(format!("Assistant: {}", m.body)),
                _ => None,
            })
            .collect();
        if lines.is_empty() {
            return Ok(None);
        }
        let recent = lines
            .iter()
            .skip(lines.len().saturating_sub(CHAT_CONTEXT_MESSAGES))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        Ok(Some(format!("Recent conversation in this space:\n{recent}")))
    }
}
