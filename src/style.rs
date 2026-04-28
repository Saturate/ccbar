pub const RST: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

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

pub fn fmt_duration(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        let h = s / 3600;
        let m = (s % 3600) / 60;
        format!("{h}h{m}m")
    } else if s >= 60 {
        let m = s / 60;
        let rem = s % 60;
        format!("{m}m{rem}s")
    } else {
        format!("{s}s")
    }
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
