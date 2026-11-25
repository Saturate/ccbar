use std::time::{SystemTime, UNIX_EPOCH};

/// Current Unix time in seconds.
///
/// Honors `CCBAR_NOW_SECS` so replay scripts and tests can drive a deterministic
/// timeline without real `sleep`s. Falls back to the wall clock when unset or
/// unparseable.
pub fn now_secs() -> i64 {
    if let Ok(v) = std::env::var("CCBAR_NOW_SECS")
        && let Ok(n) = v.parse::<i64>()
    {
        return n;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
