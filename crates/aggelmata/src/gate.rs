// Reusable concurrency admission gate whose limit is supplied live per call.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::Notify;

/// An admission gate whose limit is read fresh on every call instead of
/// fixed at construction (contrast `tokio::sync::Semaphore`, whose permit
/// count is set once). Callers thread a live limit — typically a
/// `horismos::Section<T>::get()` read, or a plain `usize` — through
/// `try_enter`/`enter` so a config reload changes admission without
/// rebuilding the gate. `aggelmata` takes no dependency on `horismos` to
/// support this: the limit arrives as a plain `usize` or closure, never a
/// config type.
pub struct LiveGate {
    in_flight: AtomicUsize,
    notify: Notify,
}

impl Default for LiveGate {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveGate {
    pub fn new() -> Self {
        Self {
            in_flight: AtomicUsize::new(0),
            notify: Notify::new(),
        }
    }

    /// Number of currently-admitted (not yet dropped) guards.
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }

    /// Admission-refusal entry: `Some(guard)` when `in_flight < limit` at the
    /// moment of the call, `None` otherwise. Never blocks; a refused call
    /// never increments the counter.
    ///
    /// INVARIANT: the check-and-increment is one atomic operation
    /// (`fetch_update`'s internal CAS retry loop), so two concurrent callers
    /// racing at `in_flight == limit - 1` cannot both observe success —
    /// exactly one wins the last slot and the other is refused.
    pub fn try_enter(self: &Arc<Self>, limit: usize) -> Option<GateGuard> {
        self.in_flight
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .ok()
            .map(|_| GateGuard {
                gate: Arc::clone(self),
            })
    }

    /// Wait-until-below entry: on every attempt (including every wake),
    /// `limit()` is re-read before re-checking admission — so a config
    /// reload that raises or lowers the effective cap is observed by the
    /// next admission decision without rebuilding the gate.
    ///
    /// WAKE ORDERING (the subtle bit): a raised limit is only re-evaluated
    /// when this waiter is woken, and the only wake source is a `GateGuard`
    /// drop (`notify_waiters()`). A limit raise with no subsequent guard
    /// drop leaves an already-parked waiter parked — there is no dedicated
    /// `poke()` to force a re-check out of band, by design (the gate stays a
    /// two-method primitive). Every real caller of `enter` (concurrency
    /// admission under ongoing traffic) pairs a raise with request traffic
    /// that drops guards continuously, so the window is not observable in
    /// practice; a caller with wholly idle traffic across a raise can force
    /// a re-check by taking and immediately dropping one extra `try_enter`.
    ///
    /// SAFETY (no missed wakeup): `self.notify.notified()` is constructed
    /// BEFORE `try_enter` is (re)attempted on each loop iteration. Per
    /// `tokio::sync::Notify`'s documented idiom, a `notify_waiters()` call
    /// that races between our failed `try_enter` and our `.await` on
    /// `notified` is still observed — the `Notified` future captures the
    /// notify generation at construction time, not at first poll, so it
    /// cannot miss a `notify_waiters()` that happens after it was created.
    pub async fn enter(self: &Arc<Self>, limit: impl Fn() -> usize) -> GateGuard {
        loop {
            let notified = self.notify.notified();
            if let Some(guard) = self.try_enter(limit()) {
                return guard;
            }
            notified.await;
        }
    }
}

/// Held while admitted through `LiveGate::try_enter`/`enter`. Dropping
/// releases the slot and wakes any `enter` waiters to re-check the
/// (possibly-changed) limit.
pub struct GateGuard {
    gate: Arc<LiveGate>,
}

impl Drop for GateGuard {
    fn drop(&mut self) {
        // INVARIANT: exactly one decrement per admitted guard — `try_enter`
        // increments exactly once per `Some` it returns, `GateGuard` is not
        // `Clone`, and `Drop` runs exactly once per value, so this fires
        // exactly once per admission.
        self.gate.in_flight.fetch_sub(1, Ordering::AcqRel);
        self.gate.notify.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn try_enter_two_concurrent_at_limit_one_exactly_one_succeeds() {
        let gate = Arc::new(LiveGate::new());
        let first = gate.try_enter(1);
        let second = gate.try_enter(1);

        assert!(first.is_some(), "first entry under the limit must succeed");
        assert!(
            second.is_none(),
            "second entry at the limit must be refused"
        );
        assert_eq!(gate.in_flight(), 1);
    }

    #[test]
    fn try_enter_refuses_without_incrementing_when_at_limit() {
        let gate = Arc::new(LiveGate::new());
        let _held = gate.try_enter(1).expect("first entry admitted");

        assert!(gate.try_enter(1).is_none());
        assert_eq!(
            gate.in_flight(),
            1,
            "a refused try_enter must not increment the counter"
        );
    }

    #[test]
    fn guard_drop_lets_next_try_enter_succeed() {
        let gate = Arc::new(LiveGate::new());
        let held = gate.try_enter(1).expect("first entry admitted");
        assert!(gate.try_enter(1).is_none());

        drop(held);

        assert!(
            gate.try_enter(1).is_some(),
            "dropping the only guard must free the slot"
        );
    }

    #[tokio::test]
    async fn enter_blocks_at_limit_then_admits_after_a_drop() {
        let gate = Arc::new(LiveGate::new());
        let held = gate.try_enter(1).expect("first entry admitted");

        let waiter_gate = Arc::clone(&gate);
        let waiter = tokio::spawn(async move { waiter_gate.enter(|| 1).await });

        // Bounded window: while the only slot stays held, the waiter must
        // never report done (no over-admission).
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(5)).await;
            assert!(
                !waiter.is_finished(),
                "waiter must not be admitted while the sole slot is held"
            );
        }

        drop(held);

        let admitted = tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .expect("waiter must be admitted once the guard drops")
            .expect("waiter task must not panic");
        assert_eq!(gate.in_flight(), 1);
        drop(admitted);
        assert_eq!(gate.in_flight(), 0);
    }

    #[tokio::test]
    async fn raised_limit_admits_a_previously_blocked_waiter_after_a_guard_drop() {
        let gate = Arc::new(LiveGate::new());
        let limit = Arc::new(AtomicUsize::new(1));

        let held = gate.try_enter(1).expect("first entry admitted at limit 1");

        let waiter_gate = Arc::clone(&gate);
        let waiter_limit = Arc::clone(&limit);
        let waiter = tokio::spawn(async move {
            waiter_gate
                .enter(|| waiter_limit.load(Ordering::Acquire))
                .await
        });

        // While the limit is still 1 and the sole slot is held, the waiter
        // must stay blocked — this also holds regardless of the raise
        // below, since it is checked strictly before the raise happens.
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(5)).await;
            assert!(
                !waiter.is_finished(),
                "waiter must not admit under the old limit"
            );
        }

        // Raise the limit. `held` is still held (in_flight == 1); a fresh
        // try_enter can now admit a SECOND concurrent guard at the raised
        // cap — this is only possible because the raise took effect, proving
        // it is live (not a one-shot construction-time value).
        limit.store(2, Ordering::Release);
        let second = gate
            .try_enter(2)
            .expect("raised limit admits a second concurrent guard");
        assert_eq!(gate.in_flight(), 2);

        // Drop the SECOND guard — not the guard the waiter is parked behind —
        // to prove ANY guard drop, not a specific one, wakes the waiter to
        // re-check the raised limit.
        drop(second);

        let admitted = tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .expect("waiter must be admitted once the raised limit is (re)checked on drop")
            .expect("waiter task must not panic");

        // `held` (the original guard) and the admitted waiter now coexist —
        // impossible under the original limit of 1.
        assert_eq!(gate.in_flight(), 2);
        drop(held);
        drop(admitted);
        assert_eq!(gate.in_flight(), 0);
    }
}
