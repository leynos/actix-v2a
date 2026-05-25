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
//! that buffer.
//!
//! `with_tracing_buffer` wraps each call under test in
//! `tracing::subscriber::with_default`, scoping capture hermetically to that
//! call's execution on the current thread. Because no `DefaultGuard` escapes
//! the closure, concurrent test threads each have their own independent
//! thread-local subscriber and the buffer contains only events from the
//! function under test.
//!
//! `tests/sse_wire_contract_bdd.rs` wires this module into the `World`
//! through `log_buffer`, calls `with_tracing_buffer` around extraction paths
//! that should emit tracing events, and uses `logs_contain` from step
//! functions to assert on the captured output.

use std::{
    fmt,
    sync::{Arc, Mutex, PoisonError},
};

use rstest_bdd::Slot;
use tracing_subscriber::{Layer, layer::SubscriberExt};

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

/// Execute `f` with a `BufferLayer` subscriber active, capturing all tracing
/// events emitted during the call into `log_buffer`.
///
/// The subscriber is installed with `tracing::subscriber::with_default` and is
/// scoped exclusively to the duration of `f`. No tracing from code outside `f`
/// is captured. `log_buffer` must already contain the `LogBuffer` for this
/// scenario; if the slot is not yet populated this function is a no-op wrapper.
pub(crate) fn with_tracing_buffer<F, T>(log_buffer: &Slot<LogBuffer>, f: F) -> T
where
    F: FnOnce() -> T,
{
    let Some(buffer) = log_buffer.get() else {
        return f();
    };
    let subscriber = tracing_subscriber::registry().with(BufferLayer(Arc::clone(&buffer)));

    tracing::subscriber::with_default(subscriber, f)
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
