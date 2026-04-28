use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext) -> Option<String> {
    let name = ctx.status.model_short();
    if name.is_empty() {
        return None;
    }

    let color = color_for_model(&name);
    let label = ctx.status.context_label();

    Some(format!("{color}{name}{RST}{DIM}/{label}{RST}"))
}
