use crate::config::BlockConfig;
use crate::style::*;

use super::{BlockPart, RenderContext};

pub fn render_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    let in_t = fmt_tokens(ctx.status.context_window.total_input_tokens);
    let out_t = fmt_tokens(ctx.status.context_window.total_output_tokens);

    let in_color = bc.color_for("input", FG_GREEN);
    let out_color = bc.color_for("output", FG_YELLOW);

    vec![
        ("input", format!("{in_color}{in_t}{RST}")),
        ("output", format!("{out_color}{out_t}{RST}")),
    ]
}
