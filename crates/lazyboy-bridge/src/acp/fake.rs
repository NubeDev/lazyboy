use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use super::{Decision, GooseClient, SessionId, Update};
use crate::BridgeError;

/// An in-process Goose double. Scripts a per-session queue of updates
/// drained by `next_update`, and records the prompts, decisions, and
/// loads the bridge sent so a test can assert on them. No real model,
/// no transport — the contract, exercised deterministically.
#[derive(Default)]
pub struct FakeGoose {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    updates: HashMap<String, VecDeque<Update>>,
    loaded: Vec<String>,
    prompts: Vec<(String, String)>,
    answers: Vec<(String, String, Decision)>,
    /// When set, the next transport call fails, simulating a dropped
    /// goosed mid-run.
    drop_next: bool,
}

impl FakeGoose {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-load the updates a session will yield, in order. Call before
    /// the driver starts pulling.
    pub fn script(&self, session: &SessionId, updates: impl IntoIterator<Item = Update>) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .updates
            .entry(session.0.clone())
            .or_default()
            .extend(updates);
    }

    /// Make the next transport call (`prompt`/`next_update`/`answer`)
    /// fail once, modelling a crash.
    pub fn drop_next(&self) {
        self.inner.lock().unwrap().drop_next = true;
    }

    pub fn loaded_sessions(&self) -> Vec<String> {
        self.inner.lock().unwrap().loaded.clone()
    }

    pub fn answers(&self) -> Vec<(String, String, Decision)> {
        self.inner.lock().unwrap().answers.clone()
    }

    fn trip(inner: &mut Inner) -> Result<(), BridgeError> {
        if std::mem::take(&mut inner.drop_next) {
            return Err(BridgeError::Transport("simulated goosed drop".into()));
        }
        Ok(())
    }
}

#[async_trait]
impl GooseClient for FakeGoose {
    async fn new_session(&self) -> Result<SessionId, BridgeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        Ok(SessionId(format!("fake-sess-{}", inner.next_id)))
    }

    async fn load_session(&self, session: &SessionId) -> Result<(), BridgeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.loaded.push(session.0.clone());
        Ok(())
    }

    async fn prompt(&self, session: &SessionId, text: &str) -> Result<(), BridgeError> {
        let mut inner = self.inner.lock().unwrap();
        Self::trip(&mut inner)?;
        inner.prompts.push((session.0.clone(), text.to_owned()));
        Ok(())
    }

    async fn next_update(&self, session: &SessionId) -> Result<Option<Update>, BridgeError> {
        let mut inner = self.inner.lock().unwrap();
        Self::trip(&mut inner)?;
        Ok(inner
            .updates
            .get_mut(&session.0)
            .and_then(VecDeque::pop_front))
    }

    async fn answer_permission(
        &self,
        session: &SessionId,
        request_id: &str,
        decision: Decision,
    ) -> Result<(), BridgeError> {
        let mut inner = self.inner.lock().unwrap();
        Self::trip(&mut inner)?;
        inner
            .answers
            .push((session.0.clone(), request_id.to_owned(), decision));
        Ok(())
    }
}
