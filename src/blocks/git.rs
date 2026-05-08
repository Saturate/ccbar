use crate::config::BlockConfig;
use crate::style::*;

use super::{BlockPart, RenderContext};

pub fn render_branch_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    let branch = match ctx.git.branch.as_deref() {
        Some(b) => b,
        None => return vec![],
    };

    let branch_color = bc.color_for("branch", FG_BMAGENTA);
    let icon_color = bc.color_for("icon", FG_BMAGENTA);

    let mut parts: Vec<BlockPart> = vec![
        ("icon", format!("{icon_color}\u{e0a0}{RST}")),
        ("branch", format!("{branch_color}{branch}{RST}")),
    ];

    if ctx.git.dirty {
        let dirty_color = bc.color_for("dirty", FG_YELLOW);
        parts.push(("dirty", format!("{dirty_color}!{RST}")));
    }

    let mut ab = String::new();
    if ctx.git.ahead > 0 {
        ab.push_str(&format!("↑{}", ctx.git.ahead));
    }
    if ctx.git.behind > 0 {
        ab.push_str(&format!("↓{}", ctx.git.behind));
    }
    if !ab.is_empty() {
        let ab_color = bc.color_for("ahead-behind", FG_CYAN);
        parts.push(("ahead-behind", format!("{ab_color}{ab}{RST}")));
    }

    parts
}

pub fn render_status_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    if ctx.git.branch.is_none() {
        return vec![];
    }

    let mut parts: Vec<BlockPart> = vec![];

    if ctx.git.dirty {
        let dirty_color = bc.color_for("dirty", FG_YELLOW);
        parts.push(("dirty", format!("{dirty_color}!{RST}")));
    }

    let mut ab = String::new();
    if ctx.git.ahead > 0 {
        ab.push_str(&format!("↑{}", ctx.git.ahead));
    }
    if ctx.git.behind > 0 {
        ab.push_str(&format!("↓{}", ctx.git.behind));
    }
    if !ab.is_empty() {
        let ab_color = bc.color_for("ahead-behind", FG_CYAN);
        parts.push(("ahead-behind", format!("{ab_color}{ab}{RST}")));
    }

    parts
}
