//! Unix timestamps with checked conversion to the database's signed seconds.
//!
//! ```
//! use mus_backend::time::{now, now_nanos, system_time_secs};
//! use std::time::{Duration, UNIX_EPOCH};
//! assert_eq!(system_time_secs(UNIX_EPOCH + Duration::from_secs(42)), Some(42));
//! assert_eq!(system_time_secs(UNIX_EPOCH - Duration::from_secs(1)), None);
//! let _current_seconds = now();
//! let _current_nanoseconds = now_nanos();
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns current Unix seconds, or zero if the clock is outside the supported range.
pub fn now() -> i64 {
    system_time_secs(SystemTime::now()).unwrap_or_default()
}

/// Returns whole Unix seconds, rejecting pre-epoch times and values beyond `i64`.
pub fn system_time_secs(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_secs()).ok())
}

/// Returns current Unix nanoseconds, or zero for a pre-epoch clock.
///
/// This is a timestamp, not a monotonic clock or a guaranteed unique identifier.
pub fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
