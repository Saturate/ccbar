use crate::config::BlockConfig;
use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext, bc: &BlockConfig) -> Option<String> {
    let cost = ctx.status.total_cost();
    let warn = bc.warn.unwrap_or(1.0);
    let crit = bc.crit.unwrap_or(5.0);
    let color = color_for_cost(cost, warn, crit);

    Some(format!("{color}${cost:.2}{RST}"))
}
