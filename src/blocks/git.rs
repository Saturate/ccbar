use crate::style::*;

use super::RenderContext;

pub fn render_branch(ctx: &RenderContext) -> Option<String> {
    let branch = ctx.git.branch.as_deref()?;
    let mut out = format!("{FG_BMAGENTA} {branch}{RST}");

    if ctx.git.dirty {
        out.push_str(&format!("{FG_YELLOW} !{RST}"));
    }

    let mut ab = String::new();
    if ctx.git.ahead > 0 {
        ab.push_str(&format!("↑{}", ctx.git.ahead));
    }
    if ctx.git.behind > 0 {
        ab.push_str(&format!("↓{}", ctx.git.behind));
    }
    if !ab.is_empty() {
        out.push_str(&format!(" {FG_CYAN}{ab}{RST}"));
    }

    Some(out)
}

pub fn render_status(ctx: &RenderContext) -> Option<String> {
    // Kept as a standalone block for configs that want branch and status separated.
    // Default config uses git-branch which now includes status inline.
    if ctx.git.branch.is_none() {
        return None;
    }

    let mut parts = String::new();

    if ctx.git.dirty {
        parts.push_str(&format!("{FG_YELLOW}!{RST}"));
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
