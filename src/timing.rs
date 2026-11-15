//! Opt-in phase timing for performance diagnosis (`TODO_TEST_TIMING=1`).

use std::io::Write as _;
use std::sync::OnceLock;
use std::time::Instant;

static ENABLED: OnceLock<bool> = OnceLock::new();

/// Returns `true` when `TODO_TEST_TIMING=1` (or `true`) is set.
#[inline]
pub(crate) fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var_os("TODO_TEST_TIMING").is_some_and(|v| v == "1" || v == "true")
    })
}

/// Logs elapsed wall time of a named phase to stderr when timing is enabled.
pub(crate) struct PhaseTimer {
    start: Option<Instant>,
    name: &'static str,
}

impl PhaseTimer {
    #[inline]
    pub(crate) fn start(name: &'static str) -> Self {
        Self {
            start: enabled().then(Instant::now),
            name,
        }
    }

    #[inline]
    pub(crate) fn finish(mut self) {
        self.emit();
    }

    fn emit(&mut self) {
        if let Some(t0) = self.start.take() {
            let us = t0.elapsed().as_micros();
            let _ = writeln!(
                std::io::stderr(),
                "[timing] {}: {}.{:03} ms",
                self.name,
                us / 1000,
                us % 1000
            );
        }
    }
}

impl Drop for PhaseTimer {
    fn drop(&mut self) {
        self.emit();
    }
}
