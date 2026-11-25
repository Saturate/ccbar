use crate::clock::now_secs;
use crate::config::BlockConfig;
use crate::status::PromptCache;
use crate::style::*;

use super::{BlockPart, RenderContext};

pub fn render_parts(ctx: &RenderContext, _bc: &BlockConfig) -> Vec<BlockPart> {
    match &ctx.status.prompt_cache {
        Some(pc) => parts(pc, now_secs()),
        None => vec![],
    }
}

fn parts(pc: &PromptCache, now: i64) -> Vec<BlockPart> {
    // Caching off (or unreported by the provider): a countdown would be noise.
    if pc.caching_observed == Some(false) {
        return vec![];
    }

    // Claude Code re-runs the statusline at expires_at, but compare against the
    // clock anyway: `warm` was true as of the last stdin, not necessarily now.
    let left = match (pc.warm, pc.expires_at) {
        (Some(true), Some(exp)) => exp.saturating_sub(now),
        _ => 0,
    };
    let state = if left <= 0 {
        format!("{FG_BCYAN}cache cold{RST}")
    } else {
        // Yellow in the last fifth of the TTL, so there is time to send first.
        let color = match ttl_secs(pc.ttl.as_deref()) {
            Some(ttl) if left.saturating_mul(5) <= ttl => FG_BYELLOW,
            _ => FG_BGREEN,
        };
        format!("{color}cache {}{RST}", fmt_left(left))
    };

    let mut out = vec![("state", state)];
    if let Some(r) = pc.hit_ratio {
        out.push((
            "ratio",
            fmt_ratio((r.clamp(0.0, 1.0) * 100.0).round() as u64),
        ));
    }
    out
}

fn ttl_secs(ttl: Option<&str>) -> Option<i64> {
    match ttl? {
        "5m" => Some(300),
        "1h" => Some(3600),
        _ => None,
    }
}

fn fmt_ratio(pct: u64) -> String {
    let color = if pct >= 80 {
        FG_BGREEN
    } else if pct >= 50 {
        FG_BYELLOW
    } else {
        FG_BRED
    };
    format!("{color}{pct}%{RST}")
}

pub fn fmt_left(secs: i64) -> String {
    if secs >= 3600 {
        format!("{}h{:02}", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc(json: &str) -> PromptCache {
        serde_json::from_str(json).unwrap()
    }

    fn plain(parts: &[BlockPart]) -> Vec<String> {
        parts.iter().map(|(_, s)| strip_ansi(s)).collect()
    }

    #[test]
    fn warm_shows_time_left_and_ratio() {
        let p = pc(
            r#"{"warm":true,"caching_observed":true,"ttl":"1h","expires_at":4000,"hit_ratio":0.914}"#,
        );
        assert_eq!(plain(&parts(&p, 1480)), ["cache 42m", "91%"]);
        assert!(parts(&p, 1480)[0].1.contains(FG_BGREEN));
    }

    #[test]
    fn last_fifth_of_ttl_is_yellow() {
        let p = pc(r#"{"warm":true,"ttl":"5m","expires_at":1000}"#);
        assert!(parts(&p, 950)[0].1.contains(FG_BYELLOW)); // 50s of 300s left
        assert!(parts(&p, 700)[0].1.contains(FG_BGREEN));
    }

    #[test]
    fn past_expiry_is_cold_even_if_warm_flag_is_stale() {
        let p = pc(r#"{"warm":true,"ttl":"5m","expires_at":1000,"hit_ratio":0.5}"#);
        assert_eq!(plain(&parts(&p, 1000)), ["cache cold", "50%"]);
    }

    #[test]
    fn not_warm_is_cold() {
        let p = pc(r#"{"warm":false,"caching_observed":true,"expires_at":null,"hit_ratio":null}"#);
        assert_eq!(plain(&parts(&p, 0)), ["cache cold"]);
    }

    #[test]
    fn hidden_when_caching_is_off() {
        let p = pc(r#"{"warm":false,"caching_observed":false}"#);
        assert!(parts(&p, 0).is_empty());
    }

    #[test]
    fn extreme_timestamps_do_not_overflow() {
        let p = pc(r#"{"warm":true,"ttl":"5m","expires_at":9223372036854775807}"#);
        assert_eq!(parts(&p, i64::MIN).len(), 1);
    }

    #[test]
    fn formats_time_left() {
        assert_eq!(fmt_left(3600), "1h00");
        assert_eq!(fmt_left(2520), "42m");
        assert_eq!(fmt_left(45), "45s");
    }
}
