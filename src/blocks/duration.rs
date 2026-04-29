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
