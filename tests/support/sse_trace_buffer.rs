//! World-owned tracing buffer support for SSE behavioural tests.

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use rstest_bdd::Slot;
use tracing_subscriber::{Layer, prelude::*};

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

        if let Ok(mut buffer) = self.0.lock() {
            buffer.push(collector.0.trim_start().to_owned());
        }
    }
}

pub(crate) struct TracingGuard(Mutex<Option<tracing::subscriber::DefaultGuard>>);

impl fmt::Debug for TracingGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TracingGuard(..)")
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.0.lock() {
            let _ = guard.take();
        }
    }
}

pub(crate) fn install_tracing(log_buffer: &Slot<LogBuffer>, tracing_guard: &Slot<TracingGuard>) {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(BufferLayer(Arc::clone(&buffer)));
    let guard = TracingGuard(Mutex::new(Some(tracing::subscriber::set_default(
        subscriber,
    ))));

    log_buffer.set(buffer);
    tracing_guard.set(guard);
}

pub(crate) fn logs_contain(log_buffer: &Slot<LogBuffer>, value: &str) -> bool {
    let Some(buffer) = log_buffer.get() else {
        return false;
    };

    buffer
        .lock()
        .is_ok_and(|lines| lines.iter().any(|line| line.contains(value)))
}
