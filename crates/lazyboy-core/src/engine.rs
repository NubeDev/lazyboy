use std::collections::HashMap;
use std::sync::Mutex;

use lazyboy_bridge::GooseClient;
use lazyboy_store::Store;
use lazyboy_types::domain::{AgentRun, Approval, Identity};
use lazyboy_types::Id;

/// Owns everything a single-node space needs to run the agent: the
/// SQLite store, a Goose client, and the agent principal that authors
/// imported timeline messages.
///
/// The two in-memory maps are recoverable presentation/coordination
/// state, never the source of truth (SCOPE.md R1). `run_seq` is the
/// per-run event counter; `pending_request` correlates a captured
/// approval to the live ACP request id so a human decision can be
/// answered. Both are rebuilt from SQLite by `reconcile` after a crash.
pub struct Engine<G: GooseClient> {
    pub(crate) store: Store,
    pub(crate) goose: G,
    pub(crate) agent_identity: Id<Identity>,
    pub(crate) state: Mutex<EngineState>,
}

#[derive(Default)]
pub(crate) struct EngineState {
    pub(crate) run_seq: HashMap<Id<AgentRun>, i64>,
    pub(crate) pending_request: HashMap<Id<Approval>, String>,
}

impl<G: GooseClient> Engine<G> {
    pub fn new(store: Store, goose: G, agent_identity: Id<Identity>) -> Self {
        Self {
            store,
            goose,
            agent_identity,
            state: Mutex::new(EngineState::default()),
        }
    }

    /// Hand out the next event seq for a run. Monotonic across drive
    /// and resume because it is keyed by run id, surviving as long as
    /// the process does and rebuilt from the run's event count on
    /// reconcile.
    pub(crate) fn next_seq(&self, run: Id<AgentRun>) -> i64 {
        let mut state = self.state.lock().unwrap();
        let seq = state.run_seq.entry(run).or_insert(0);
        *seq += 1;
        *seq
    }

    /// Seed a run's seq counter to a known high-water mark, so a
    /// re-drive after a crash starts numbering past the events already
    /// imported (see `reconcile`).
    pub(crate) fn set_seq(&self, run: Id<AgentRun>, seq: i64) {
        self.state.lock().unwrap().run_seq.insert(run, seq);
    }

    pub(crate) fn remember_request(&self, approval: Id<Approval>, request_id: String) {
        self.state
            .lock()
            .unwrap()
            .pending_request
            .insert(approval, request_id);
    }

    pub(crate) fn take_request(&self, approval: Id<Approval>) -> Option<String> {
        self.state.lock().unwrap().pending_request.remove(&approval)
    }

    pub fn store(&self) -> &Store {
        &self.store
    }
}
