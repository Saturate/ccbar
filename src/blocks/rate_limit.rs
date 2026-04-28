use crate::config::BlockConfig;
use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext, bc: &BlockConfig) -> Option<String> {
    let rl = ctx.status.rate_limits.as_ref()?;
    let bar_width = bc.bar_width.unwrap_or(8);
    let show_countdown = bc.show_countdown.unwrap_or(true);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut parts = Vec::new();

    if let Some(five) = &rl.five_hour {
        if let Some(pct) = five.used_percentage {
            let pct_int = pct as u64;
            let bar = progress_bar(pct_int, bar_width);
            let mut segment = String::new();

            if show_countdown {
                if let Some(resets) = five.resets_at {
                    let remaining = resets - now;
                    let cd = fmt_countdown(remaining);
                    segment.push_str(&format!("{FG_MAGENTA} {cd}{RST} "));
                }
            }

            segment.push_str(&format!("{bar} {DIM}{pct_int}%{RST}"));
            parts.push(segment);
        }
    }

    if let Some(seven) = &rl.seven_day {
        if let Some(pct) = seven.used_percentage {
            let pct_int = pct as u64;
            let bar = progress_bar(pct_int, bar_width);
            let mut segment = String::new();

            if show_countdown {
                if let Some(resets) = seven.resets_at {
                    let remaining = resets - now;
                    let cd = fmt_countdown(remaining);
                    segment.push_str(&format!("{FG_MAGENTA} {cd}{RST} "));
                }
            }

            segment.push_str(&format!("{bar} {DIM}{pct_int}%{RST}"));
            parts.push(segment);
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("  "))
    }
}
