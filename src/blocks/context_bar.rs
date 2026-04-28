use crate::config::BlockConfig;
use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext, bc: &BlockConfig) -> Option<String> {
    let pct = ctx.status.context_pct();
    let width = bc.width.unwrap_or(12);

    let bar = progress_bar(pct, width);
    Some(format!("{FG_CYAN}ctx{RST} {bar} {DIM}{pct}%{RST}"))
}
