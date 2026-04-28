use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext) -> Option<String> {
    let ms = ctx.status.duration_ms();
    if ms == 0 {
        return None;
    }
    let s = fmt_duration(ms);
    Some(format!("{DIM}{s}{RST}"))
}
