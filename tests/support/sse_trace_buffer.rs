//! World-owned tracing buffer support for SSE behavioural tests.
//!
//! This module exists so the BDD scenarios can assert on emitted tracing
//! events without relying on `tracing-test` 0.2.x. That crate exposes only the
//! internal, undocumented `tracing_test::internal::logs_with_scope_contain`
//! helper for assertions outside a `#[traced_test]` body.
//!
//! The buffer model is intentionally simple: each BDD `World` owns a
//! `LogBuffer = Arc<Mutex<Vec<String>>>`; `BufferLayer` implements
//! `tracing_subscriber::Layer` and captures every tracing event's fields into
//! that buffer for the duration of the scenario; and `TracingGuard` holds the
//! `DefaultGuard` returned by `tracing::subscriber::set_default` so the
//! subscriber is deregistered when the guard drops.
//!
//! `tests/sse_wire_contract_bdd.rs` wires this module into the `World`
//! through `log_buffer` and `tracing_guard`, calls `install_tracing` from the
//! `world()` fixture, and uses `logs_contain` from step functions to assert on
//! the captured output.

use std::{
    fmt,
    sync::{Arc, Mutex, PoisonError},
};

use rstest_bdd::Slot;
use tracing_subscriber::{Layer, prelude::*};

/// Thread-safe in-memory log storage for one BDD `World`.
///
/// The buffer is an `Arc<Mutex<Vec<String>>>` so the tracing layer and the
/// `World` can share captured event strings safely.
pub(crate) type LogBuffer = Arc<Mutex<Vec<String>>>;

struct BufferLayer(LogBuffer);

impl<S: tracing::Subscriber> Layer<S> for BufferLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        use tracing_subscriber::field::Visit;

        struct Collector(String);

        impl Visit for Collector {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
                use fmt::Write as _;

                let result = write!(self.0, " {}={:?}", field.name(), value);
                debug_assert!(result.is_ok(), "writing to String should not fail");
            }
        }

        let mut collector = Collector(String::new());
        event.record(&mut collector);

        let mut buffer = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        buffer.push(collector.0.trim_start().to_owned());
    }
}

/// Holds the installed default tracing subscriber for one BDD `World`.
///
/// The inner `DefaultGuard` comes from `set_default` and is stored inside a
/// `Mutex` so the guard wrapper is `Send + Sync`. Dropping this guard
/// deregisters the subscriber and restores the previous default subscriber.
pub(crate) struct TracingGuard(Mutex<Option<tracing::subscriber::DefaultGuard>>);

impl fmt::Debug for TracingGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TracingGuard(..)")
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        let mut guard = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = guard.take();
    }
}

/// Install the tracing subscriber used by the SSE BDD world.
///
/// The `log_buffer` slot receives a fresh `LogBuffer`, and the `tracing_guard`
/// slot receives the `TracingGuard` that keeps the `BufferLayer`-backed
/// subscriber installed for the scenario.
pub(crate) fn install_tracing(log_buffer: &Slot<LogBuffer>, tracing_guard: &Slot<TracingGuard>) {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(BufferLayer(Arc::clone(&buffer)));
    let guard = TracingGuard(Mutex::new(Some(tracing::subscriber::set_default(
        subscriber,
    ))));

    log_buffer.set(buffer);
    tracing_guard.set(guard);
}

/// Check whether the world-owned trace buffer contains `value`.
///
/// The `log_buffer` slot provides the scenario's trace buffer. This performs a
/// substring match against each captured event line, returns `false` when the
/// slot has not yet been initialised, and recovers poisoned locks so a prior
/// panic does not silently hide the captured output.
pub(crate) fn logs_contain(log_buffer: &Slot<LogBuffer>, value: &str) -> bool {
    let Some(buffer) = log_buffer.get() else {
        return false;
    };

    let lines = buffer.lock().unwrap_or_else(PoisonError::into_inner);
    lines.iter().any(|line| line.contains(value))
}
