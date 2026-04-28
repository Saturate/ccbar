use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext) -> Option<String> {
    let in_t = fmt_tokens(ctx.status.context_window.total_input_tokens);
    let out_t = fmt_tokens(ctx.status.context_window.total_output_tokens);
    Some(format!("{FG_GREEN}{in_t}{RST} {FG_YELLOW}{out_t}{RST}"))
}
