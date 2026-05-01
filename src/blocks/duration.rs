use crate::config::BlockConfig;
use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext, bc: &BlockConfig) -> Option<String> {
    let ms = ctx.status.duration_ms();
    if ms == 0 {
        return None;
    }

    let s = match &bc.format {
        Some(fmt) => fmt_duration_custom(ms, fmt),
        None => fmt_duration(ms),
    };

    Some(format!("{DIM}{s}{RST}"))
}

fn fmt_duration_custom(ms: u64, fmt: &str) -> String {
    let total_s = ms / 1000;
    let d = total_s / 86400;
    let h = (total_s % 86400) / 3600;
    let m = (total_s % 3600) / 60;
    let s = total_s % 60;
    let total_h = total_s / 3600;
    let total_m = total_s / 60;

    fmt.replace("{d}", &d.to_string())
        .replace("{h}", &h.to_string())
        .replace("{m}", &m.to_string())
        .replace("{s}", &s.to_string())
        .replace("{total_h}", &total_h.to_string())
        .replace("{total_m}", &total_m.to_string())
        .replace("{total_s}", &total_s.to_string())
        .replace("{h:02}", &format!("{h:02}"))
        .replace("{m:02}", &format!("{m:02}"))
        .replace("{s:02}", &format!("{s:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_format_all_tokens() {
        let ms = 691_092_000; // 7d 23h 58m 12s
        assert_eq!(fmt_duration_custom(ms, "{d}d {h}h {m}m {s}s"), "7d 23h 58m 12s");
    }

    #[test]
    fn custom_format_total_hours() {
        let ms = 691_092_000;
        assert_eq!(fmt_duration_custom(ms, "{total_h}:{m:02}:{s:02}"), "191:58:12");
    }

    #[test]
    fn custom_format_zero_padded() {
        let ms = 3_661_000; // 1h 1m 1s
        assert_eq!(fmt_duration_custom(ms, "{h:02}:{m:02}:{s:02}"), "01:01:01");
    }

    #[test]
    fn custom_format_hours_minutes_only() {
        let ms = 86_280_000; // 23h 58m
        assert_eq!(fmt_duration_custom(ms, "{h}h{m}m"), "23h58m");
    }

    #[test]
    fn custom_format_total_seconds() {
        let ms = 5_000;
        assert_eq!(fmt_duration_custom(ms, "{total_s}s"), "5s");
    }

    #[test]
    fn custom_format_total_minutes() {
        let ms = 300_000; // 5m
        assert_eq!(fmt_duration_custom(ms, "{total_m}m"), "5m");
    }
}
