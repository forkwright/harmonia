//! Lifecycle op registry for the MCP stdio surface (#652, PR 4 of the rmcp
//! migration) — long-lived playback and renderer work represented as STATE
//! instead of an indefinitely open RPC.
//!
//! One slot per work kind on `HarmoniaServer` (`playback`, `renderer`); a
//! slot holds at most one running op plus the exit summary of the most
//! recent one. The six `*_start`/`*_status`/`*_stop` tools in `mcp.rs` are
//! thin envelopes over `OpSlot`.
//!
//! Semantics (pinned by the tests below):
//! - `start` spawns the work under a FRESH CancellationToken owned by the
//!   slot — deliberately independent of the starting request's
//!   `RequestContext.ct`, because the op must outlive the RPC that created
//!   it. A second start while an op runs is REFUSED with the running op's
//!   id; the registry never silently replaces live work.
//! - `status` reports running / exited-with-summary / idle. With no `op_id`
//!   it describes the running op, else the most recent exit, else idle; an
//!   `op_id` must name one of those two or the call is a tool error (only
//!   the running and most-recent op are queryable by id).
//! - `stop` cancels the op's token and awaits its teardown, then records
//!   the exit. With no `op_id` it targets the running op; an `op_id` naming
//!   anything else is refused without touching the live op. Stopping when
//!   nothing runs is a clean tool error.
//! - Exit reaping is lazy: a naturally-finished op is joined and its summary
//!   recorded by the next status/start call. `JoinHandle::is_finished` gates
//!   that await, so the slot lock is never held across a pending join.
//!
//! Op ids are `{kind}-{n}`, monotonic within one server process. The stop
//! await is unbounded on purpose: PR 3 proved both work loops tear down
//! promptly on cancellation (the play.rs and runner.rs falsifier tests), so
//! a wedged teardown is a bug to surface, not a timeout to paper over.

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use archon::mcp_params::{PlayFileParams, RenderParams};

use crate::cli::PlayArgs;

/// The terminal record of a spawned op: `Ok` carries the work's own exit
/// summary, `Err` its failure text. Plain strings — the status/stop
/// envelopes serialize them verbatim.
pub(crate) type OpOutcome = Result<String, String>;

/// A registry op identifier (`{kind}-{n}`, monotonic per server process).
/// Newtype so a lifecycle op id cannot be silently crossed with the other
/// stringly ids on the MCP surface (download queue rows, want UUIDs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpId(String);

impl OpId {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OpId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The recorded exit of a finished op.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpExit {
    pub(crate) op_id: OpId,
    pub(crate) summary: String,
}

/// What `status` learned about a slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpStatus {
    /// Nothing is running and no op has ever completed.
    Idle,
    /// An op is still running.
    Running { op_id: OpId },
    /// The named op finished; `summary` is its own exit or failure text.
    Exited { op_id: OpId, summary: String },
}

struct RunningOp {
    op_id: OpId,
    stop: CancellationToken,
    join: JoinHandle<OpOutcome>,
}

impl RunningOp {
    /// Joins the work task and folds its outcome into the recorded summary.
    /// A panicked or aborted task is reported in the summary, never
    /// propagated — the registry must stay usable after a bad op.
    async fn into_exit(self) -> OpExit {
        let summary = match self.join.await {
            Ok(Ok(summary)) => summary,
            Ok(Err(failure)) => format!("failed: {failure}"),
            Err(e) => format!("the work task did not join cleanly: {e}"),
        };
        OpExit {
            op_id: self.op_id,
            summary,
        }
    }
}

/// One lifecycle slot: at most one running op of a kind, plus the last exit.
pub(crate) struct OpSlot {
    kind: &'static str,
    next_seq: u64,
    running: Option<RunningOp>,
    last_exit: Option<OpExit>,
}

impl OpSlot {
    pub(crate) fn new(kind: &'static str) -> Self {
        Self {
            kind,
            next_seq: 0,
            running: None,
            last_exit: None,
        }
    }

    /// Starts `spawn` under a fresh slot-owned token and returns the new op
    /// id. A finished-but-unreaped op is reaped first, so start-after-exit
    /// works without an intervening status call; a genuinely running op
    /// refuses the start.
    pub(crate) async fn start(
        &mut self,
        spawn: impl FnOnce(CancellationToken) -> JoinHandle<OpOutcome>,
    ) -> Result<OpId, String> {
        self.reap_finished().await;
        if let Some(running) = &self.running {
            return Err(format!(
                "{} is already running as op {}; stop it before starting another",
                self.kind, running.op_id
            ));
        }
        self.next_seq += 1;
        let op_id = OpId(format!("{}-{}", self.kind, self.next_seq));
        let stop = CancellationToken::new();
        let join = spawn(stop.clone());
        self.running = Some(RunningOp {
            op_id: op_id.clone(),
            stop,
            join,
        });
        Ok(op_id)
    }

    pub(crate) async fn status(&mut self, op_id: Option<&str>) -> Result<OpStatus, String> {
        self.reap_finished().await;
        match op_id {
            Some(id) => {
                if let Some(running) = &self.running
                    && running.op_id.as_str() == id
                {
                    return Ok(OpStatus::Running {
                        op_id: running.op_id.clone(),
                    });
                }
                if let Some(exit) = &self.last_exit
                    && exit.op_id.as_str() == id
                {
                    return Ok(OpStatus::Exited {
                        op_id: exit.op_id.clone(),
                        summary: exit.summary.clone(),
                    });
                }
                Err(format!(
                    "unknown {} op {id}: it is neither the running op nor the most recent exit",
                    self.kind
                ))
            }
            None => {
                if let Some(running) = &self.running {
                    Ok(OpStatus::Running {
                        op_id: running.op_id.clone(),
                    })
                } else if let Some(exit) = &self.last_exit {
                    Ok(OpStatus::Exited {
                        op_id: exit.op_id.clone(),
                        summary: exit.summary.clone(),
                    })
                } else {
                    Ok(OpStatus::Idle)
                }
            }
        }
    }

    /// Cancels the running op's token and awaits its teardown. A mismatched
    /// `op_id` refuses WITHOUT vacating the slot — the live op survives a
    /// client that meant to stop something else.
    pub(crate) async fn stop(&mut self, op_id: Option<&str>) -> Result<OpExit, String> {
        let Some(running) = self.running.take() else {
            return Err(format!("no {} op is running", self.kind));
        };
        if let Some(id) = op_id
            && running.op_id.as_str() != id
        {
            let current = running.op_id.clone();
            self.running = Some(running);
            return Err(format!(
                "{} op {id} is not the running op ({current}); refusing to stop",
                self.kind
            ));
        }
        running.stop.cancel();
        let exit = running.into_exit().await;
        self.last_exit = Some(exit.clone());
        Ok(exit)
    }

    /// Joins a naturally-finished op and records its exit. The `is_finished`
    /// gate means the await never pends on live work, so callers may hold
    /// the slot lock across this without stalling on a running op.
    async fn reap_finished(&mut self) {
        let finished = self
            .running
            .as_ref()
            .is_some_and(|op| op.join.is_finished());
        if !finished {
            return;
        }
        if let Some(running) = self.running.take() {
            self.last_exit = Some(running.into_exit().await);
        }
    }
}

/// Spawns one playback of `params` under `stop`, the registry-owned token a
/// `*_stop` call cancels. The `Ok` summary is the play loop's own output —
/// which already names a cancellation (play.rs writes "playback stopped
/// (cancelled)") — or "playback completed" when the track ran out silently.
pub(crate) fn spawn_playback(
    params: PlayFileParams,
    stop: CancellationToken,
) -> JoinHandle<OpOutcome> {
    tokio::spawn(async move {
        let mut out: Vec<u8> = Vec::new();
        let result = crate::play::run_play(
            PlayArgs {
                file: params.file,
                device: params.device,
            },
            &mut out,
            stop,
        )
        .await;
        match result {
            Ok(()) => {
                // WHY lossy: this is a human-readable exit summary, not a
                // wire contract — one lossy line beats a third error path.
                let text = String::from_utf8_lossy(&out).trim().to_string();
                if text.is_empty() {
                    Ok("playback completed".to_string())
                } else {
                    Ok(text)
                }
            }
            Err(e) => Err(e.to_string()),
        }
    })
}

/// Spawns the renderer loop under `stop` — same shape as `spawn_playback`.
pub(crate) fn spawn_renderer(
    params: RenderParams,
    stop: CancellationToken,
) -> JoinHandle<OpOutcome> {
    let observed = stop.clone();
    tokio::spawn(async move {
        let result = crate::render::run_render(
            crate::render::RenderArgs {
                server: params.server,
                cert_dir: params
                    .cert_dir
                    .unwrap_or_else(crate::paths::default_renderer_cert_dir),
                name: params.name,
                config_path: params.config,
            },
            stop,
        )
        .await;
        match result {
            // WHY: run_render returns Ok on both natural exit and a registry
            // stop (its shutdown tree treats both as a clean drain); only the
            // slot's token tells them apart, so name the stop cause here.
            Ok(()) if observed.is_cancelled() => Ok("renderer stopped (cancelled)".to_string()),
            Ok(()) => Ok("renderer exited".to_string()),
            Err(e) => Err(e.to_string()),
        }
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use super::*;

    /// A synthetic op that pends until its registry token fires — mirrors
    /// the select structure of the real play/render loops without touching
    /// audio hardware or the network.
    fn spawn_synthetic(
        stop: CancellationToken,
        cancelled: Arc<AtomicBool>,
    ) -> JoinHandle<OpOutcome> {
        tokio::spawn(async move {
            stop.cancelled().await;
            cancelled.store(true, Ordering::SeqCst);
            Ok("synthetic op stopped".to_string())
        })
    }

    #[tokio::test]
    async fn status_reports_idle_before_anything_runs() {
        let mut slot = OpSlot::new("playback");
        assert_eq!(slot.status(None).await.unwrap(), OpStatus::Idle);
        // An op id is a tool error even before the first start.
        assert!(slot.status(Some("playback-1")).await.is_err());
    }

    #[tokio::test]
    async fn start_assigns_monotonic_op_ids_and_refuses_a_second_start() {
        let mut slot = OpSlot::new("test");
        let first = slot
            .start(|ct| spawn_synthetic(ct, Arc::new(AtomicBool::new(false))))
            .await
            .unwrap();
        assert_eq!(first.as_str(), "test-1");

        let err = slot
            .start(|ct| spawn_synthetic(ct, Arc::new(AtomicBool::new(false))))
            .await
            .unwrap_err();
        assert!(err.contains("already running"), "{err}");
        // The refusal names the live op so the client can stop or poll it.
        assert!(err.contains("test-1"), "{err}");

        slot.stop(None).await.unwrap();
        let second = slot
            .start(|ct| spawn_synthetic(ct, Arc::new(AtomicBool::new(false))))
            .await
            .unwrap();
        assert_eq!(second.as_str(), "test-2");
        slot.stop(None).await.unwrap();
    }

    #[tokio::test]
    async fn stop_without_a_running_op_is_a_clean_error() {
        let mut slot = OpSlot::new("playback");
        let err = slot.stop(None).await.unwrap_err();
        assert!(err.contains("no playback op is running"), "{err}");
        let err = slot.stop(Some("playback-9")).await.unwrap_err();
        assert!(err.contains("no playback op is running"), "{err}");
    }

    #[tokio::test]
    async fn stop_cancels_the_op_and_status_reports_the_exit() {
        let mut slot = OpSlot::new("playback");
        let cancelled = Arc::new(AtomicBool::new(false));
        let op_id = slot
            .start(|ct| spawn_synthetic(ct, cancelled.clone()))
            .await
            .unwrap();

        assert_eq!(
            slot.status(None).await.unwrap(),
            OpStatus::Running {
                op_id: op_id.clone()
            }
        );

        let exit = slot.stop(Some(op_id.as_str())).await.unwrap();
        assert_eq!(exit.op_id, op_id);
        assert!(exit.summary.contains("synthetic op stopped"), "{exit:?}");
        // WHY: the point of the registry — the stop token IS the task's
        // token, so the work itself observed the cancellation.
        assert!(cancelled.load(Ordering::SeqCst));

        assert_eq!(
            slot.status(None).await.unwrap(),
            OpStatus::Exited {
                op_id: op_id.clone(),
                summary: "synthetic op stopped".to_string()
            }
        );
        // The exited op stays queryable by id…
        assert!(matches!(
            slot.status(Some(op_id.as_str())).await.unwrap(),
            OpStatus::Exited { .. }
        ));
        // …but stopping it again is a clean error, not a no-op success.
        assert!(slot.stop(Some(op_id.as_str())).await.is_err());
    }

    #[tokio::test]
    async fn status_reaps_a_naturally_finished_op_and_start_can_replace_it() {
        let mut slot = OpSlot::new("test");
        let op_id = slot
            .start(|_ct| tokio::spawn(async { Ok("done on its own".to_string()) }))
            .await
            .unwrap();

        // Poll until the lazy reap lands — the task's exact finish timing is
        // the runtime's, the reap is the next status call after that.
        let summary = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match slot.status(Some(op_id.as_str())).await.unwrap() {
                    OpStatus::Exited { summary, .. } => break summary,
                    _ => tokio::time::sleep(Duration::from_millis(10)).await,
                }
            }
        })
        .await
        .expect("a finished op must be reaped by status");
        assert!(summary.contains("done on its own"), "{summary}");

        // The slot is free again without an explicit stop.
        let second = slot
            .start(|_ct| tokio::spawn(async { Ok("second".to_string()) }))
            .await
            .unwrap();
        assert_eq!(second.as_str(), "test-2");
    }

    #[tokio::test]
    async fn stop_with_a_mismatched_op_id_refuses_and_keeps_the_op_running() {
        let mut slot = OpSlot::new("renderer");
        let cancelled = Arc::new(AtomicBool::new(false));
        let op_id = slot
            .start(|ct| spawn_synthetic(ct, cancelled.clone()))
            .await
            .unwrap();

        let err = slot.stop(Some("renderer-99")).await.unwrap_err();
        assert!(err.contains("renderer-99"), "{err}");
        assert!(err.contains(op_id.as_str()), "{err}");

        // The refusal left the live op untouched.
        assert!(!cancelled.load(Ordering::SeqCst));
        assert_eq!(
            slot.status(None).await.unwrap(),
            OpStatus::Running {
                op_id: op_id.clone()
            }
        );

        // And the correctly-addressed stop still works.
        slot.stop(Some(op_id.as_str())).await.unwrap();
        assert!(cancelled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn only_the_running_and_most_recent_op_are_queryable_by_id() {
        let mut slot = OpSlot::new("test");
        let first = slot
            .start(|ct| spawn_synthetic(ct, Arc::new(AtomicBool::new(false))))
            .await
            .unwrap();
        slot.stop(None).await.unwrap();
        let second = slot
            .start(|ct| spawn_synthetic(ct, Arc::new(AtomicBool::new(false))))
            .await
            .unwrap();
        slot.stop(None).await.unwrap();

        // One exit deep: the older op id is now unknown, not a stale exit.
        assert!(matches!(
            slot.status(Some(second.as_str())).await.unwrap(),
            OpStatus::Exited { .. }
        ));
        let err = slot.status(Some(first.as_str())).await.unwrap_err();
        assert!(err.contains("unknown test op"), "{err}");
    }

    #[tokio::test]
    async fn a_failed_op_is_recorded_as_a_failed_exit() {
        let mut slot = OpSlot::new("playback");
        let op_id = slot
            .start(|_ct| tokio::spawn(async { Err("engine blew up".to_string()) }))
            .await
            .unwrap();
        let summary = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match slot.status(Some(op_id.as_str())).await.unwrap() {
                    OpStatus::Exited { summary, .. } => break summary,
                    _ => tokio::time::sleep(Duration::from_millis(10)).await,
                }
            }
        })
        .await
        .expect("a failed op must be reaped by status");
        assert!(summary.contains("failed: engine blew up"), "{summary}");
    }

    #[tokio::test]
    async fn spawn_playback_of_a_missing_file_exits_promptly() {
        // WHY: the registry-to-run_play seam without hardware. A missing
        // file fails fast on any machine — either Engine setup or the decode
        // task errors — so the op must EXIT promptly carrying the failure,
        // never hang the slot. The Ok/Err split is the engine's environment-
        // dependent choice; both are exits with a non-empty summary.
        let join = spawn_playback(
            PlayFileParams {
                file: std::path::PathBuf::from("/nonexistent/harmonia-mcp-ops.flac"),
                device: None,
            },
            CancellationToken::new(),
        );
        let outcome = tokio::time::timeout(Duration::from_secs(10), join)
            .await
            .expect("a missing file must not hang the playback op")
            .expect("the playback task must not panic");
        match outcome {
            Ok(summary) => assert!(!summary.is_empty()),
            Err(failure) => assert!(!failure.is_empty()),
        }
    }
}
