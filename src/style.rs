pub const RST: &str = "\x1b[0m";
#[allow(dead_code)]
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

#[allow(dead_code)]
pub const FG_RED: &str = "\x1b[31m";
pub const FG_GREEN: &str = "\x1b[32m";
pub const FG_YELLOW: &str = "\x1b[33m";
pub const FG_MAGENTA: &str = "\x1b[35m";
pub const FG_CYAN: &str = "\x1b[36m";

pub const FG_BRED: &str = "\x1b[91m";
pub const FG_BGREEN: &str = "\x1b[92m";
pub const FG_BYELLOW: &str = "\x1b[93m";
pub const FG_BCYAN: &str = "\x1b[96m";
pub const FG_BMAGENTA: &str = "\x1b[95m";

pub fn resolve_color(name: &str) -> &'static str {
    match name {
        "red" => FG_RED,
        "green" => FG_GREEN,
        "yellow" => FG_YELLOW,
        "blue" => "\x1b[34m",
        "magenta" => FG_MAGENTA,
        "cyan" => FG_CYAN,
        "white" => "\x1b[37m",
        "bright-red" => FG_BRED,
        "bright-green" => FG_BGREEN,
        "bright-yellow" => FG_BYELLOW,
        "bright-blue" => "\x1b[94m",
        "bright-magenta" => FG_BMAGENTA,
        "bright-cyan" => FG_BCYAN,
        "bright-white" => "\x1b[97m",
        "dim" => DIM,
        _ => "",
    }
}

pub fn color_for_model(name: &str) -> &'static str {
    if name.contains("opus") {
        FG_MAGENTA
    } else if name.contains("haiku") {
        FG_GREEN
    } else {
        FG_CYAN
    }
}

pub fn color_for_pct(pct: u64) -> &'static str {
    if pct >= 90 {
        FG_BRED
    } else if pct >= 75 {
        FG_YELLOW
    } else if pct >= 50 {
        FG_BYELLOW
    } else {
        FG_BGREEN
    }
}

pub fn color_for_cost(cost: f64, warn: f64, crit: f64) -> &'static str {
    if cost > crit {
        FG_BRED
    } else if cost > warn {
        FG_BYELLOW
    } else {
        FG_BGREEN
    }
}

pub fn progress_bar(pct: u64, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    let color = color_for_pct(pct);

    let mut bar = String::with_capacity(width * 4 + 20);
    bar.push_str(color);
    for _ in 0..filled {
        bar.push('━');
    }
    bar.push_str(RST);
    bar.push_str(DIM);
    for _ in 0..empty {
        bar.push('┄');
    }
    bar.push_str(RST);
    bar
}

pub fn fmt_tokens(t: u64) -> String {
    if t >= 1_000_000 {
        format!("{:.1}M", t as f64 / 1_000_000.0)
    } else if t >= 1_000 {
        format!("{:.1}k", t as f64 / 1_000.0)
    } else {
        t.to_string()
    }
}

#[cfg(test)]
fn fmt_duration(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 86400 {
        let d = s / 86400;
        let h = (s % 86400) / 3600;
        let m = (s % 3600) / 60;
        let rem = s % 60;
        format!("{d}d{h}h{m}m{rem}s")
    } else if s >= 3600 {
        let h = s / 3600;
        let m = (s % 3600) / 60;
        let rem = s % 60;
        format!("{h}h{m}m{rem}s")
    } else if s >= 60 {
        let m = s / 60;
        let rem = s % 60;
        format!("{m}m{rem}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape && c == 'm' {
            in_escape = false;
        } else if !in_escape {
            out.push(c);
        }
    }
    out
}

pub fn fmt_countdown(secs: i64) -> String {
    if secs <= 0 {
        "now".to_string()
    } else if secs >= 86400 {
        let d = secs / 86400;
        let h = (secs % 86400) / 3600;
        format!("{d}d{h}h")
    } else if secs >= 3600 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h{m}m")
    } else {
        let m = secs / 60;
        format!("{m}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_tokens_raw() {
        assert_eq!(fmt_tokens(0), "0");
        assert_eq!(fmt_tokens(500), "500");
        assert_eq!(fmt_tokens(999), "999");
    }

    #[test]
    fn fmt_tokens_thousands() {
        assert_eq!(fmt_tokens(1_000), "1.0k");
        assert_eq!(fmt_tokens(45_200), "45.2k");
        assert_eq!(fmt_tokens(999_999), "1000.0k");
    }

    #[test]
    fn fmt_tokens_millions() {
        assert_eq!(fmt_tokens(1_000_000), "1.0M");
        assert_eq!(fmt_tokens(2_500_000), "2.5M");
    }

    #[test]
    fn fmt_duration_seconds() {
        assert_eq!(fmt_duration(0), "0s");
        assert_eq!(fmt_duration(42_000), "42s");
        assert_eq!(fmt_duration(59_000), "59s");
    }

    #[test]
    fn fmt_duration_minutes() {
        assert_eq!(fmt_duration(60_000), "1m0s");
        assert_eq!(fmt_duration(192_000), "3m12s");
    }

    #[test]
    fn fmt_duration_hours() {
        assert_eq!(fmt_duration(3_600_000), "1h0m0s");
        assert_eq!(fmt_duration(12_360_000), "3h26m0s");
    }

    #[test]
    fn fmt_duration_days() {
        assert_eq!(fmt_duration(86_400_000), "1d0h0m0s");
        assert_eq!(fmt_duration(691_092_000), "7d23h58m12s");
    }

    #[test]
    fn fmt_countdown_values() {
        assert_eq!(fmt_countdown(0), "now");
        assert_eq!(fmt_countdown(-5), "now");
        assert_eq!(fmt_countdown(300), "5m");
        assert_eq!(fmt_countdown(3661), "1h1m");
        assert_eq!(fmt_countdown(90061), "1d1h");
    }

    #[test]
    fn color_for_cost_thresholds() {
        assert_eq!(color_for_cost(0.5, 1.0, 5.0), FG_BGREEN);
        assert_eq!(color_for_cost(1.0, 1.0, 5.0), FG_BGREEN);
        assert_eq!(color_for_cost(1.01, 1.0, 5.0), FG_BYELLOW);
        assert_eq!(color_for_cost(5.01, 1.0, 5.0), FG_BRED);
    }

    #[test]
    fn color_for_pct_thresholds() {
        assert_eq!(color_for_pct(0), FG_BGREEN);
        assert_eq!(color_for_pct(49), FG_BGREEN);
        assert_eq!(color_for_pct(50), FG_BYELLOW);
        assert_eq!(color_for_pct(74), FG_BYELLOW);
        assert_eq!(color_for_pct(75), FG_YELLOW);
        assert_eq!(color_for_pct(89), FG_YELLOW);
        assert_eq!(color_for_pct(90), FG_BRED);
        assert_eq!(color_for_pct(100), FG_BRED);
    }

    #[test]
    fn color_for_model_families() {
        assert_eq!(color_for_model("opus 4.6"), FG_MAGENTA);
        assert_eq!(color_for_model("claude-opus-4-6"), FG_MAGENTA);
        assert_eq!(color_for_model("haiku 4.5"), FG_GREEN);
        assert_eq!(color_for_model("sonnet 4.6"), FG_CYAN);
        assert_eq!(color_for_model("unknown"), FG_CYAN);
    }

    #[test]
    fn progress_bar_structure() {
        let bar = strip_ansi(&progress_bar(50, 10));
        assert_eq!(bar.chars().count(), 10);
        assert_eq!(bar.chars().filter(|&c| c == '━').count(), 5);
        assert_eq!(bar.chars().filter(|&c| c == '┄').count(), 5);
    }

    #[test]
    fn progress_bar_extremes() {
        let empty = strip_ansi(&progress_bar(0, 8));
        assert_eq!(empty.chars().filter(|&c| c == '━').count(), 0);

        let full = strip_ansi(&progress_bar(100, 8));
        assert_eq!(full.chars().filter(|&c| c == '━').count(), 8);
    }

    #[test]
    fn resolve_color_known() {
        assert_eq!(resolve_color("red"), FG_RED);
        assert_eq!(resolve_color("green"), FG_GREEN);
        assert_eq!(resolve_color("cyan"), FG_CYAN);
        assert_eq!(resolve_color("bright-magenta"), FG_BMAGENTA);
        assert_eq!(resolve_color("dim"), DIM);
    }

    #[test]
    fn resolve_color_unknown_returns_empty() {
        assert_eq!(resolve_color("neon-pink"), "");
        assert_eq!(resolve_color(""), "");
    }
}
