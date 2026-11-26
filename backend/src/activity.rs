//! Tracks whether the service is being used so background workers can stop
//! polling the database when it is idle. With a scale-to-zero database (Neon)
//! a worker that queries every few seconds keeps compute awake forever; gating
//! the polling on real traffic lets it suspend between lectures.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::time::Duration;

use axum::{extract::Request, middleware::Next, response::Response};
use tokio::sync::Notify;

static LAST_ACTIVITY: AtomicI64 = AtomicI64::new(0);
static STARTED: AtomicI64 = AtomicI64::new(0);
static PENDING_BATCHES: AtomicUsize = AtomicUsize::new(0);
static FORCE_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Woken when the service goes from idle to busy so slow-polling workers react at once.
static WAKE: Notify = Notify::const_new();

/// Workers keep polling this long after the last request.
const ACTIVE_WINDOW_SECS: i64 = 600;
/// A gap this long between requests counts as "idle" for wake-up purposes.
const IDLE_AFTER_SECS: i64 = 120;
/// After a restart, do one round of catch-up work regardless of traffic.
const STARTUP_GRACE_SECS: i64 = 180;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn init() {
    STARTED.store(now(), Ordering::Relaxed);
}

pub fn touch() {
    let current = now();
    let previous = LAST_ACTIVITY.swap(current, Ordering::Relaxed);
    if current - previous >= IDLE_AFTER_SECS {
        WAKE.notify_waiters();
    }
}

/// Sleeps for the adaptive poll interval, or until activity resumes after an
/// idle period, whichever comes first.
pub async fn sleep(base: Duration) {
    tokio::select! {
        _ = tokio::time::sleep(tick(base)) => {}
        _ = WAKE.notified() => {}
    }
}

pub fn set_pending_batches(count: usize) {
    PENDING_BATCHES.store(count, Ordering::Relaxed);
}

#[cfg(test)]
pub fn force_active(value: bool) {
    FORCE_ACTIVE.store(value, Ordering::Relaxed);
}

/// Should a worker query the database this tick?
pub fn db_active() -> bool {
    if FORCE_ACTIVE.load(Ordering::Relaxed) || PENDING_BATCHES.load(Ordering::Relaxed) > 0 {
        return true;
    }
    let current = now();
    current - LAST_ACTIVITY.load(Ordering::Relaxed) < ACTIVE_WINDOW_SECS
        || current - STARTED.load(Ordering::Relaxed) < STARTUP_GRACE_SECS
}

/// Poll quickly while people are around, slowly while only a batch is pending.
pub fn tick(base: Duration) -> Duration {
    let current = now();
    if current - LAST_ACTIVITY.load(Ordering::Relaxed) < 120 {
        base
    } else if PENDING_BATCHES.load(Ordering::Relaxed) > 0 {
        base * 20
    } else {
        base * 4
    }
}

/// Request middleware: any call except health probes counts as activity.
pub async fn track(request: Request, next: Next) -> Response {
    if !request.uri().path().starts_with("/health") && request.uri().path() != "/" {
        touch();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test: the counters are process-wide, so separate tests would race.
    #[tokio::test]
    async fn idle_behaviour_slows_polling_and_first_request_wakes_workers() {
        LAST_ACTIVITY.store(now(), Ordering::Relaxed);
        PENDING_BATCHES.store(0, Ordering::Relaxed);
        assert_eq!(tick(Duration::from_secs(10)), Duration::from_secs(10));
        LAST_ACTIVITY.store(now() - 500, Ordering::Relaxed);
        assert_eq!(tick(Duration::from_secs(10)), Duration::from_secs(40));
        PENDING_BATCHES.store(2, Ordering::Relaxed);
        assert_eq!(tick(Duration::from_secs(10)), Duration::from_secs(200));
        PENDING_BATCHES.store(0, Ordering::Relaxed);

        LAST_ACTIVITY.store(0, Ordering::Relaxed); // long idle
        let waiter = tokio::spawn(async { sleep(Duration::from_secs(3600)).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        touch();
        tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("worker should wake immediately")
            .unwrap();
    }
}
