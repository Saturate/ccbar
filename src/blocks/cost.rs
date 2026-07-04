use crate::config::BlockConfig;
use crate::currency;
use crate::style::*;

use super::{BlockPart, RenderContext};

pub fn render_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    let cost_usd = ctx.status.total_cost();
    let currency = bc.currency.as_deref().unwrap_or("USD");

    let (cost, prefix) = if currency.eq_ignore_ascii_case("USD") {
        (cost_usd, "$".to_string())
    } else {
        match currency::get_rate(currency) {
            Some(rate) => {
                let sym = currency::symbol(currency);
                let prefix = if sym.is_empty() {
                    format!("{} ", currency.to_uppercase())
                } else {
                    sym.to_string()
                };
                (cost_usd * rate, prefix)
            }
            None => (cost_usd, "$".to_string()),
        }
    };

    let warn = bc.warn.unwrap_or(1.0);
    let crit = bc.crit.unwrap_or(5.0);
    let default_color = color_for_cost(cost, warn, crit);
    let color = bc.color_for("cost", default_color);

    vec![("cost", format!("{color}{prefix}{cost:.2}{RST}"))]
}
