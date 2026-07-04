use crate::config::BlockConfig;
use crate::style::*;

use super::{BlockPart, RenderContext};

pub fn render_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    let name = ctx.status.model_short();
    if name.is_empty() {
        return vec![];
    }

    let default_color = color_for_model(&name);
    let name_color = bc.color_for("name", default_color);
    let size_color = bc.color_for("context-size", DIM);
    let label = ctx.status.context_label();

    vec![
        ("name", format!("{name_color}{name}{RST}")),
        ("context-size", format!("{size_color}/{label}{RST}")),
    ]
}
