use crate::blocks::{render_block, RenderContext};
use crate::config::Config;
use crate::style::{DIM, RST};

pub fn render_lines(config: &Config, ctx: &RenderContext) -> Vec<String> {
    let sep = format!("{DIM}{}{RST}", config.separator.char);

    config
        .lines
        .iter()
        .filter_map(|line| {
            let rendered: Vec<String> = line
                .blocks
                .iter()
                .filter_map(|name| render_block(name, ctx))
                .collect();

            if rendered.is_empty() {
                None
            } else {
                Some(format!("\x1b[0m{}", rendered.join(&sep)))
            }
        })
        .collect()
}
