use crate::config::BlockConfig;
use crate::style::*;

use super::{BlockPart, RenderContext};

pub fn render_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    let pct = ctx.status.context_pct();
    let width = bc.width.unwrap_or(12);
    let bar = progress_bar(pct, width);

    let label_color = bc.color_for("label", FG_CYAN);
    let pct_color = bc.color_for("pct", DIM);

    vec![
        ("label", format!("{label_color}ctx{RST}")),
        ("bar", bar),
        ("pct", format!("{pct_color}{pct}%{RST}")),
    ]
}
