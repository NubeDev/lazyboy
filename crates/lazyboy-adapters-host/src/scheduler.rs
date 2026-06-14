//! The stage-C schedule tick: a host-side clock that fires due
//! schedule-triggered workflows (SCOPE.md "Workflows and automation",
//! schedule trigger; DOCS/GOOSE-FEATURES.md "Scheduler"). It lives in
//! this host-only crate, not in `lazyboy-core`, because it owns a timer
//! and (through the engine factory) the live goose transport — the same
//! reason process spawn is confined here. The mobile-safe crate graph
//! never reaches it.
//!
//! The clock is deliberately thin: every tick it asks the engine's pure
//! `dispatch_schedule_tick` which schedules are due in the window since
//! the last tick, and each due workflow runs through the ordinary gated
//! `run_workflow` drive loop — same durable approval row, same
//! per-workflow approval policy as an interactive run (R6). We do NOT
//! call goose's own scheduler, which would run around the gate.

use std::future::Future;
use std::time::Duration;

use time::OffsetDateTime;

use lazyboy_bridge::{BridgeError, GooseClient};
use lazyboy_core::{Engine, ScheduleTickReport};

/// A host-side schedule clock. Construct with the poll interval, then
/// either drive it tick-by-tick (`tick_once`, for tests) or spawn it as
/// a background task (`spawn`).
///
/// `since` is the exclusive lower bound of the next window — the instant
/// the previous tick observed. The engine's `dispatch_schedule_tick`
/// fires each cron-matching minute in `(since, now]` exactly once, so a
/// late or long tick never double-fires and never skips a minute. `since`
/// starts at construction time, so a schedule due in the very first
/// interval is caught.
pub struct Scheduler {
    interval: Duration,
    since: OffsetDateTime,
}

impl Scheduler {
    /// `interval` is how often the clock polls. A minute or less keeps
    /// minute-granularity cron entries timely; the window math means a
    /// coarser interval still fires every matching minute, just later.
    /// `start` is the first window's exclusive lower bound (pass
    /// `OffsetDateTime::now_utc()` in production).
    pub fn new(interval: Duration, start: OffsetDateTime) -> Self {
        Self {
            interval,
            since: start,
        }
    }

    /// Run one tick against `engine` as of `now`: fire the schedules due
    /// in `(self.since, now]`, then advance `since` to `now`. Returns the
    /// report so a caller (or test) can observe what fired and what was
    /// skipped. Advancing `since` only after a successful tick means a
    /// transient failure re-considers the same window next time rather
    /// than silently dropping a due minute.
    pub async fn tick_once<G: GooseClient>(
        &mut self,
        engine: &Engine<G>,
        now: OffsetDateTime,
    ) -> Result<ScheduleTickReport, BridgeError> {
        let report = engine
            .dispatch_schedule_tick(self.since, now)
            .await
            .map_err(|e| BridgeError::Transport(format!("schedule tick: {e}")))?;
        self.since = now;
        Ok(report)
    }

    /// Spawn the clock as a background task. Each tick builds a fresh
    /// engine through `make_engine` (mirroring how the server builds one
    /// per mutating request, since the host transport is per-connection),
    /// runs `tick_once`, and logs the outcome. A tick whose engine cannot
    /// be built — goose down, no provider — is logged and retried next
    /// interval; it does not kill the clock. The returned handle aborts
    /// the task when dropped.
    pub fn spawn<G, F, Fut>(mut self, make_engine: F) -> SchedulerHandle
    where
        G: GooseClient + 'static,
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Engine<G>, BridgeError>> + Send,
    {
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(self.interval);
            // The first immediate fire would use a zero-width window
            // (since == start == now), so it harmlessly fires nothing;
            // subsequent ticks carry real windows.
            loop {
                ticker.tick().await;
                let now = OffsetDateTime::now_utc();
                match make_engine().await {
                    Ok(engine) => match self.tick_once(&engine, now).await {
                        Ok(report) => {
                            if !report.fired.is_empty() || !report.skipped.is_empty() {
                                tracing::info!(
                                    fired = report.fired.len(),
                                    skipped = report.skipped.len(),
                                    "schedule tick fired workflows"
                                );
                            }
                        }
                        Err(e) => tracing::warn!(%e, "schedule tick failed; retrying next interval"),
                    },
                    Err(e) => {
                        tracing::warn!(%e, "schedule tick skipped; engine unavailable")
                    }
                }
            }
        });
        SchedulerHandle { handle }
    }
}

/// Owns the spawned clock task and aborts it on drop, so a shell that
/// drops the handle (shutdown) stops the clock cleanly.
pub struct SchedulerHandle {
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for SchedulerHandle {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
