use crate::style::*;

use super::RenderContext;

pub fn render_branch(ctx: &RenderContext) -> Option<String> {
    let branch = ctx.git.branch.as_deref()?;
    Some(format!("{FG_BMAGENTA} {branch}{RST}"))
}

pub fn render_status(ctx: &RenderContext) -> Option<String> {
    if ctx.git.branch.is_none() {
        return None;
    }

    let mut parts = String::new();

    if ctx.git.dirty {
        parts.push_str(&format!("{FG_YELLOW} !{RST}"));
    }

    if ctx.git.ahead > 0 {
        parts.push_str(&format!("{FG_CYAN}↑{}{RST}", ctx.git.ahead));
    }
    if ctx.git.behind > 0 {
        parts.push_str(&format!("{FG_CYAN}↓{}{RST}", ctx.git.behind));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}
