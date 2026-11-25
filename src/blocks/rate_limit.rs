use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::clock::now_secs;
use crate::config::BlockConfig;
use crate::fsutil::{cache_dir, stable_hash, write_atomic};
use crate::status::RateLimits;
use crate::style::*;

use super::{BlockPart, RenderContext};

/// Shared across all ccbar instances. Whichever tab gets fresh rate_limits
/// writes here; any tab missing it reads the cached value.
#[derive(Serialize, Deserialize, Default, Clone)]
struct CachedLimits {
    updated_at: i64,
    five_pct: Option<f64>,
    five_resets: Option<i64>,
    seven_pct: Option<f64>,
    seven_resets: Option<i64>,
    #[serde(default)]
    spend_pct: Option<f64>,
    #[serde(default)]
    spend_resets: Option<i64>,
    #[serde(default)]
    spend_used_usd: Option<f64>,
    #[serde(default)]
    spend_limit_usd: Option<f64>,
    #[serde(default)]
    spend_period: Option<String>,
}

impl CachedLimits {
    #[allow(clippy::type_complexity)]
    fn data(&self) -> ([Option<f64>; 5], [Option<i64>; 3], &Option<String>) {
        (
            [
                self.five_pct,
                self.seven_pct,
                self.spend_pct,
                self.spend_used_usd,
                self.spend_limit_usd,
            ],
            [self.five_resets, self.seven_resets, self.spend_resets],
            &self.spend_period,
        )
    }

    fn from_status(rl: &RateLimits, now: i64) -> Self {
        let spend = rl.spend_limit.as_ref();
        CachedLimits {
            updated_at: now,
            five_pct: rl.five_hour.as_ref().and_then(|f| f.used_percentage),
            five_resets: rl.five_hour.as_ref().and_then(|f| f.resets_at),
            seven_pct: rl.seven_day.as_ref().and_then(|s| s.used_percentage),
            seven_resets: rl.seven_day.as_ref().and_then(|s| s.resets_at),
            spend_pct: spend.and_then(|s| s.used_percentage),
            spend_resets: spend.and_then(|s| s.resets_at),
            spend_used_usd: spend.and_then(|s| s.used_usd),
            spend_limit_usd: spend.and_then(|s| s.limit_usd),
            spend_period: spend.and_then(|s| s.period.clone()),
        }
    }
}

/// Pick between this tab's view of one window and the cached one. Tabs whose
/// session has been idle keep sending the percentage from their last response,
/// so a plain overwrite would let them roll the cache back. Within one window
/// usage only rises, so the same `resets_at` keeps the higher value, and a later
/// `resets_at` means a newer window.
/// Windows are 5h or longer, so reset times this close belong to one window;
/// this tolerates the server's timestamp shifting by a few seconds.
const SAME_WINDOW_SLACK: i64 = 60;

fn merge_window(
    old: (Option<f64>, Option<i64>),
    new: (Option<f64>, Option<i64>),
    now: i64,
) -> (Option<f64>, Option<i64>) {
    let (Some(op), Some(np)) = (old.0, new.0) else {
        return if new.0.is_some() { new } else { old };
    };
    match (old.1, new.1) {
        (Some(or), _) if or <= now => new,
        (Some(or), Some(nr)) if nr > or + SAME_WINDOW_SLACK => new,
        (Some(or), Some(nr)) if nr + SAME_WINDOW_SLACK < or => old,
        _ => (Some(op.max(np)), new.1.or(old.1)),
    }
}

fn merge(old: CachedLimits, new: CachedLimits, now: i64) -> CachedLimits {
    let (five_pct, five_resets) = merge_window(
        (old.five_pct, old.five_resets),
        (new.five_pct, new.five_resets),
        now,
    );
    let (seven_pct, seven_resets) = merge_window(
        (old.seven_pct, old.seven_resets),
        (new.seven_pct, new.seven_resets),
        now,
    );
    let (spend_pct, spend_resets) = merge_window(
        (old.spend_pct, old.spend_resets),
        (new.spend_pct, new.spend_resets),
        now,
    );
    // The USD figures arrive on a slower poll; within one period keep the
    // cached ones until this tab has its own. Across periods they'd be wrong.
    let near = |a: Option<i64>, b: Option<i64>| match (a, b) {
        (Some(a), Some(b)) => a.abs_diff(b) <= SAME_WINDOW_SLACK as u64,
        _ => a == b,
    };
    let same_period = near(old.spend_resets, new.spend_resets);
    let spend_from_new = spend_resets == new.spend_resets;
    let pick = |n: Option<f64>, o: Option<f64>| match (same_period, spend_from_new) {
        (true, _) => n.or(o),
        (false, true) => n,
        (false, false) => o,
    };
    // Spend only grows within a period, so an idle tab's older figure must not
    // replace a higher cached one (same rule as the percentages).
    let used = match (same_period, new.spend_used_usd, old.spend_used_usd) {
        (true, Some(n), Some(o)) => Some(n.max(o)),
        _ => pick(new.spend_used_usd, old.spend_used_usd),
    };
    let spend_period = match (same_period, spend_from_new) {
        (true, _) => new.spend_period.or(old.spend_period.clone()),
        (false, true) => new.spend_period,
        (false, false) => old.spend_period.clone(),
    };
    let mut merged = CachedLimits {
        updated_at: old.updated_at,
        five_pct,
        five_resets,
        seven_pct,
        seven_resets,
        spend_pct,
        spend_resets,
        spend_used_usd: used,
        spend_limit_usd: pick(new.spend_limit_usd, old.spend_limit_usd),
        spend_period,
    };
    // Idle tabs resend the same values every render. Only data that changed the
    // cache counts as an update, so MAX_CACHE_AGE can still expire it.
    if merged.data() != old.data() {
        merged.updated_at = now;
    }
    merged
}

/// Limits also move from usage outside this machine (claude.ai, other
/// devices), so a cache nobody has refreshed for this long is too old to show.
const MAX_CACHE_AGE: i64 = 60 * 60;

/// One cache per Claude config dir: separate `CLAUDE_CONFIG_DIR`s usually mean
/// separate accounts, and one account's limits must not show in the other's tabs.
fn cache_path() -> PathBuf {
    let name = match std::env::var("CLAUDE_CONFIG_DIR") {
        Ok(dir) if !dir.is_empty() => format!("rate-limits-{:016x}.json", stable_hash(&dir)),
        _ => "rate-limits.json".to_string(),
    };
    cache_dir().join(name)
}

fn load_cache() -> Option<CachedLimits> {
    let s = fs::read_to_string(cache_path()).ok()?;
    serde_json::from_str(&s).ok()
}

/// Drop what Claude Code itself would no longer send: windows past their
/// `resets_at`, or the whole cache once it is older than MAX_CACHE_AGE.
fn fresh_cached(mut l: CachedLimits, now: i64) -> Option<CachedLimits> {
    if now.saturating_sub(l.updated_at) > MAX_CACHE_AGE {
        return None;
    }
    let expired = |resets: Option<i64>| resets.is_some_and(|r| r <= now);
    if expired(l.five_resets) {
        l.five_pct = None;
    }
    if expired(l.seven_resets) {
        l.seven_pct = None;
    }
    if expired(l.spend_resets) {
        l.spend_pct = None;
    }
    Some(l)
}

struct RenderOpts {
    bar_width: usize,
    show_countdown: bool,
    now: i64,
    from_cache: bool,
}

pub fn render_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    let now = now_secs();
    let opts = RenderOpts {
        bar_width: bc.bar_width.unwrap_or(8),
        show_countdown: bc.show_countdown.unwrap_or(true),
        now,
        from_cache: false,
    };

    // Fresh data from stdin → render it as-is and merge it into the shared cache
    if let Some(rl) = ctx.status.rate_limits.as_ref() {
        let limits = CachedLimits::from_status(rl, now);
        let path = cache_path();
        let merged = match load_cache() {
            Some(old) => merge(old, limits.clone(), now),
            None => limits.clone(),
        };
        write_atomic(&path, &merged);
        return render_limits(&limits, &opts);
    }

    // No stdin data → fall back to shared cache from any other tab
    let Some(cached) = load_cache().and_then(|l| fresh_cached(l, now)) else {
        return vec![];
    };
    render_limits(
        &cached,
        &RenderOpts {
            from_cache: true,
            ..opts
        },
    )
}

fn render_limits(l: &CachedLimits, opts: &RenderOpts) -> Vec<BlockPart> {
    let mut parts: Vec<BlockPart> = Vec::new();

    if let Some(pct) = l.five_pct {
        parts.push(("5h", render_window(pct, l.five_resets, opts)));
    }

    if let Some(pct) = l.seven_pct {
        parts.push(("7d", render_window(pct, l.seven_resets, opts)));
    }

    // Spend only matters once a subscription window is exhausted. Gateway
    // users get no 5h/7d windows at all, so spend is their only signal.
    let quota_hit = [l.five_pct, l.seven_pct]
        .iter()
        .flatten()
        .any(|&p| p >= 100.0);
    let no_windows = l.five_pct.is_none() && l.seven_pct.is_none();
    if (quota_hit || no_windows)
        && let Some(pct) = l.spend_pct
    {
        parts.push(("spend", render_spend(pct, l, opts)));
    }

    parts
}

fn render_spend(pct: f64, l: &CachedLimits, opts: &RenderOpts) -> String {
    let pct_int = pct as u64;
    let bar = progress_bar(pct_int, opts.bar_width);
    let mut segment = String::new();

    if opts.show_countdown
        && let Some(resets) = l.spend_resets
    {
        let cd = fmt_countdown(resets.saturating_sub(opts.now));
        segment.push_str(&format!("{FG_MAGENTA} {cd}{RST} "));
    }

    segment.push_str(&format!("{bar} "));
    if let (Some(used), Some(limit)) = (l.spend_used_usd, l.spend_limit_usd) {
        let color = color_for_pct(pct_int);
        segment.push_str(&format!("{color}${used:.2}{RST}{DIM}/${limit:.0}{RST} "));
    }
    segment.push_str(&format!("{DIM}{pct_int}%"));
    if let Some(p) = l.spend_period.as_deref().and_then(|p| p.chars().next()) {
        segment.push_str(&format!("/{p}"));
    }
    segment.push_str(RST);
    if opts.from_cache {
        segment.push_str(&format!("{DIM}*{RST}"));
    }
    segment
}

fn render_window(pct: f64, resets_at: Option<i64>, opts: &RenderOpts) -> String {
    let pct_int = pct as u64;
    let bar = progress_bar(pct_int, opts.bar_width);
    let mut segment = String::new();

    if opts.show_countdown
        && let Some(resets) = resets_at
    {
        let remaining = resets.saturating_sub(opts.now);
        let cd = fmt_countdown(remaining);
        segment.push_str(&format!("{FG_MAGENTA} {cd}{RST} "));
    }

    segment.push_str(&format!("{bar} {DIM}{pct_int}%{RST}"));
    if opts.from_cache {
        segment.push_str(&format!("{DIM}*{RST}"));
    }
    segment
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> RenderOpts {
        RenderOpts {
            bar_width: 4,
            show_countdown: false,
            now: 0,
            from_cache: false,
        }
    }

    fn limits(five: Option<f64>, seven: Option<f64>) -> CachedLimits {
        CachedLimits {
            five_pct: five,
            seven_pct: seven,
            spend_pct: Some(62.8),
            spend_used_usd: Some(314.12),
            spend_limit_usd: Some(500.0),
            spend_period: Some("monthly".into()),
            ..Default::default()
        }
    }

    fn render(l: &CachedLimits) -> String {
        let parts: Vec<String> = render_limits(l, &opts())
            .into_iter()
            .map(|(_, s)| s)
            .collect();
        strip_ansi(&parts.join(" "))
    }

    #[test]
    fn cached_windows_past_reset_are_dropped() {
        let l = CachedLimits {
            updated_at: 1000,
            five_pct: Some(100.0),
            five_resets: Some(2000),
            seven_pct: Some(30.0),
            seven_resets: Some(900_000),
            spend_pct: Some(40.0),
            ..Default::default()
        };
        let l = fresh_cached(l, 2500).unwrap();
        assert_eq!(l.five_pct, None);
        assert_eq!(l.seven_pct, Some(30.0));
        // A stale 100% must not trigger the spend segment either.
        let out = render(&l);
        assert!(!out.contains("40%"), "{out}");
    }

    #[test]
    fn idle_tab_cannot_roll_back_newer_usage() {
        let active = CachedLimits {
            five_pct: Some(60.0),
            five_resets: Some(9000),
            ..Default::default()
        };
        let idle = CachedLimits {
            five_pct: Some(30.0),
            five_resets: Some(9000),
            ..Default::default()
        };
        assert_eq!(merge(active, idle, 2000).five_pct, Some(60.0));
    }

    #[test]
    fn reset_jitter_counts_as_same_window() {
        let active = CachedLimits {
            five_pct: Some(60.0),
            five_resets: Some(9000),
            ..Default::default()
        };
        let idle = CachedLimits {
            five_pct: Some(30.0),
            five_resets: Some(9001),
            ..Default::default()
        };
        assert_eq!(merge(active, idle, 2000).five_pct, Some(60.0));
    }

    #[test]
    fn newer_window_replaces_older_one() {
        let old_window = CachedLimits {
            five_pct: Some(95.0),
            five_resets: Some(3000),
            ..Default::default()
        };
        let new_window = CachedLimits {
            five_pct: Some(5.0),
            five_resets: Some(21000),
            ..Default::default()
        };
        // Old window not yet expired by the clock, but the new one is later.
        let m = merge(old_window, new_window, 2000);
        assert_eq!((m.five_pct, m.five_resets), (Some(5.0), Some(21000)));
        // And an idle tab still on the old window can't bring it back.
        let stale = CachedLimits {
            five_pct: Some(95.0),
            five_resets: Some(3000),
            ..Default::default()
        };
        let m = merge(m, stale, 2001);
        assert_eq!(m.five_pct, Some(5.0));
    }

    #[test]
    fn expired_cached_window_yields_to_new_data() {
        let old = CachedLimits {
            five_pct: Some(90.0),
            five_resets: Some(1000),
            ..Default::default()
        };
        let new = CachedLimits {
            five_pct: Some(2.0),
            five_resets: None,
            ..Default::default()
        };
        assert_eq!(merge(old, new, 1500).five_pct, Some(2.0));
    }

    #[test]
    fn idle_tab_cannot_lower_cached_spend() {
        let cached = CachedLimits {
            spend_pct: Some(80.0),
            spend_resets: Some(9000),
            spend_used_usd: Some(400.0),
            spend_limit_usd: Some(500.0),
            ..Default::default()
        };
        let idle = CachedLimits {
            spend_pct: Some(20.0),
            spend_resets: Some(9000),
            spend_used_usd: Some(100.0),
            spend_limit_usd: Some(500.0),
            ..Default::default()
        };
        let m = merge(cached, idle, 2000);
        assert_eq!((m.spend_pct, m.spend_used_usd), (Some(80.0), Some(400.0)));
    }

    #[test]
    fn spend_usd_does_not_cross_periods() {
        let old = CachedLimits {
            spend_pct: Some(80.0),
            spend_resets: Some(1000),
            spend_used_usd: Some(400.0),
            spend_limit_usd: Some(500.0),
            spend_period: Some("monthly".into()),
            ..Default::default()
        };
        let new = CachedLimits {
            spend_pct: Some(1.0),
            spend_resets: Some(9000),
            ..Default::default()
        };
        let m = merge(old, new, 1500);
        assert_eq!(m.spend_pct, Some(1.0));
        assert_eq!(m.spend_used_usd, None);
    }

    #[test]
    fn repeated_values_do_not_refresh_age() {
        let old = CachedLimits {
            updated_at: 0,
            five_pct: Some(30.0),
            five_resets: Some(90_000),
            ..Default::default()
        };
        let same = CachedLimits {
            five_pct: Some(30.0),
            five_resets: Some(90_000),
            ..Default::default()
        };
        let m = merge(old, same, 4000);
        assert_eq!(m.updated_at, 0);
        assert!(fresh_cached(m, 4010).is_none());
        let old = CachedLimits {
            updated_at: 0,
            five_pct: Some(30.0),
            five_resets: Some(90_000),
            ..Default::default()
        };
        let higher = CachedLimits {
            five_pct: Some(31.0),
            five_resets: Some(90_000),
            ..Default::default()
        };
        assert_eq!(merge(old, higher, 4000).updated_at, 4000);
    }

    #[test]
    fn old_cache_is_ignored() {
        let l = CachedLimits {
            updated_at: 0,
            five_pct: Some(10.0),
            ..Default::default()
        };
        assert!(fresh_cached(l, MAX_CACHE_AGE + 1).is_none());
    }

    #[test]
    fn spend_hidden_while_quota_left() {
        let out = render(&limits(Some(40.0), Some(99.0)));
        assert!(!out.contains('$'), "{out}");
    }

    #[test]
    fn spend_shown_when_five_hour_hit() {
        let out = render(&limits(Some(100.0), Some(20.0)));
        assert!(out.contains("$314.12/$500 62%/m"), "{out}");
    }

    #[test]
    fn spend_shown_when_seven_day_hit() {
        let out = render(&limits(None, Some(100.0)));
        assert!(out.contains("$314.12/$500"), "{out}");
    }

    #[test]
    fn spend_shown_without_subscription_windows() {
        let out = render(&limits(None, None));
        assert!(out.contains("62%/m"), "{out}");
    }

    #[test]
    fn spend_without_usd_fields_shows_pct_only() {
        let mut l = limits(None, None);
        l.spend_used_usd = None;
        l.spend_period = None;
        let out = render(&l);
        assert!(!out.contains('$'), "{out}");
        assert!(out.ends_with("62%"), "{out}");
    }

    #[test]
    fn parses_spend_limit_from_status_json() {
        let json = r#"{"rate_limits":{"spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"weekly"}}}"#;
        let status: crate::status::StatusJSON = serde_json::from_str(json).unwrap();
        let l = CachedLimits::from_status(status.rate_limits.as_ref().unwrap(), 0);
        assert_eq!(l.spend_pct, Some(62.8));
        assert_eq!(l.spend_limit_usd, Some(500.0));
        assert_eq!(l.spend_period.as_deref(), Some("weekly"));
    }
}
