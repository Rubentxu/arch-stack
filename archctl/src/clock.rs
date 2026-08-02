//! Clock port — abstraction over "what time is it?".
//!
//! The domain uses timestamps in three places:
//! - `Evidence::observed_at` (RFC3339 string, persisted to graph TIMESTAMP column)
//! - any future `created_at` / `updated_at` on Element/Relation
//! - command logging that needs a stable wall-clock timestamp
//!
//! The domain does NOT call `chrono::Utc::now()` directly. Instead it
//! receives a `&dyn Clock` from the entry point and asks the port.
//! This lets tests inject a [`FixedClock`] for deterministic
//! timestamps and, if we ever need to, freeze the wall clock for
//! snapshot-based tests.
//!
//! ## What the port hides
//!
//! - **The "now" source.** Today it is `chrono::Utc`. Tomorrow it
//!   could be a `time::OffsetDateTime`, a mock, or a monotonic
//!   counter for property tests. The port is the boundary.
//!
//! - **Timezone semantics.** The string is RFC3339 (always UTC, with
//!   trailing `Z`). Callers never see a `DateTime<FixedOffset>` or
//!   wonder whether the column is local-time.
//!
//! ## What the port does NOT hide
//!
//! - **String parsing.** If a domain operation needs to read a
//!   stored timestamp back, the caller parses the RFC3339 string
//!   with whatever crate it wants. Parsing is a different concern
//!   than producing "now".
//!
//! - **Monotonic clocks.** Not modelled. If a domain use case ever
//!   needs "time since last event", add a `Clock::now_monotonic()`
//!   method behind the same trait — do not introduce a second port.

use std::sync::Arc;

/// The clock port.
///
/// Implementations:
/// - [`SystemClock`] — production adapter, calls `chrono::Utc::now()`.
/// - [`FixedClock`] — test adapter, returns a constant timestamp.
///
/// The trait returns an RFC3339-formatted string in UTC (`...Z`)
/// because that is what `evidence::observed_at` and the lbug TIMESTAMP
/// column expect. Callers must not assume any other format.
pub trait Clock: Send + Sync {
    /// Current wall-clock instant, formatted as RFC3339 with `Z` suffix.
    fn now_rfc3339(&self) -> String;
}

/// Production adapter — wraps `chrono::Utc::now()`.
///
/// Cheap to construct (zero-cost struct). The `Arc<dyn Clock>` shared
/// across the call tree is the `SystemClock` instance; cloning the
/// `Arc` does not clone the clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
}

/// Test adapter — returns a fixed timestamp on every call.
///
/// Construct with the timestamp you want every call to see:
///
/// ```
/// use archctl::clock::{Clock, FixedClock};
/// let clock = FixedClock::new("2026-07-30T00:00:00Z");
/// assert_eq!(clock.now_rfc3339(), "2026-07-30T00:00:00Z");
/// ```
#[derive(Debug, Clone)]
pub struct FixedClock {
    stamp: String,
}

impl FixedClock {
    /// Build a fixed clock that always returns `stamp`.
    ///
    /// The string is stored verbatim — the caller is responsible for
    /// ensuring it is RFC3339 (with `Z` or `+00:00`). The constructor
    /// does NOT validate the format because tests sometimes want to
    /// inject deliberately malformed timestamps to exercise the
    /// downstream behaviour.
    pub fn new(stamp: impl Into<String>) -> Self {
        Self {
            stamp: stamp.into(),
        }
    }
}

impl Clock for FixedClock {
    fn now_rfc3339(&self) -> String {
        self.stamp.clone()
    }
}

/// Factory: the production clock, type-erased to the trait.
///
/// Use this at the entry point (CLI, MCP server, future Python
/// binding) and pass `Arc<dyn Clock>` through the call tree.
pub fn system_clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}

/// Helper for tests: a fixed clock from a literal string.
pub fn fixed_clock(stamp: &str) -> Arc<dyn Clock> {
    Arc::new(FixedClock::new(stamp))
}

/// Helper: format any `chrono::DateTime<Utc>` as the same RFC3339 shape
/// the port produces. Kept private to the module to discourage using
/// it from outside — the clock port is the only sanctioned way to get
/// the current instant in domain code.
///
/// Currently unused: `SystemClock` calls `chrono::Utc::now().to_rfc3339_opts`
/// directly, so this helper is here for the day someone wants to
/// format a stored timestamp the same way. Kept `#[allow(dead_code)]`
/// to avoid a warning while staying discoverable.
#[allow(dead_code)]
fn format_chrono(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_returns_constant_string() {
        let c = FixedClock::new("2026-07-30T12:34:56Z");
        assert_eq!(c.now_rfc3339(), "2026-07-30T12:34:56Z");
        assert_eq!(c.now_rfc3339(), "2026-07-30T12:34:56Z");
    }

    #[test]
    fn system_clock_returns_rfc3339_with_z_suffix() {
        let stamp = SystemClock.now_rfc3339();
        assert!(
            stamp.ends_with('Z'),
            "SystemClock must emit trailing Z, got {stamp}"
        );
        // Sanity: round-trip parses as UTC RFC3339. We do not pull in
        // a chrono::DateTime here — instead, just confirm length and
        // a fixed prefix shape (year-month-day T hour:min:sec).
        assert_eq!(
            stamp.len(),
            20,
            "expected YYYY-MM-DDTHH:MM:SSZ, got {stamp}"
        );
        let bytes = stamp.as_bytes();
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
        assert_eq!(bytes[10], b'T');
        assert_eq!(bytes[13], b':');
        assert_eq!(bytes[16], b':');
    }

    #[test]
    fn factory_returns_dyn_clock() {
        let c = system_clock();
        let _: &dyn Clock = c.as_ref();
        let c = fixed_clock("1970-01-01T00:00:00Z");
        let _: &dyn Clock = c.as_ref();
    }

    #[test]
    fn format_chrono_matches_system_clock_shape() {
        // The helper exists for parity with SystemClock. We verify the
        // shape is identical (length + suffix) so anyone formatting a
        // stored timestamp gets the same wire format the port emits.
        let from_helper = format_chrono(chrono::Utc::now());
        assert_eq!(from_helper.len(), 20);
        assert!(from_helper.ends_with('Z'));
    }
}
