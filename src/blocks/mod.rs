pub mod context_bar;
pub mod cost;
pub mod dir;
pub mod duration;
pub mod git;
pub mod model;
pub mod rate_limit;
pub mod tokens;

use crate::config::{BlockConfig, Config};
use crate::status::StatusJSON;

pub struct GitInfo {
    pub branch: Option<String>,
    pub dirty: bool,
    pub ahead: u64,
    pub behind: u64,
}

pub struct RenderContext<'a> {
    pub status: &'a StatusJSON,
    pub git: &'a GitInfo,
    pub config: &'a Config,
}

pub fn render_block(name: &str, ctx: &RenderContext) -> Option<String> {
    let bc = ctx.config.blocks.get(name);
    let default_bc = BlockConfig::default();
    let bc = bc.unwrap_or(&default_bc);

    match name {
        "dir" => dir::render(ctx, bc),
        "git-branch" => git::render_branch(ctx),
        "git-status" => git::render_status(ctx),
        "model" => model::render(ctx),
        "context-bar" => context_bar::render(ctx, bc),
        "tokens" => tokens::render(ctx),
        "cost" => cost::render(ctx, bc),
        "duration" => duration::render(ctx, bc),
        "rate-limit" => rate_limit::render(ctx, bc),
        _ => None,
    }
}

pub fn collect_git_info(cwd: &str) -> GitInfo {
    if cwd.is_empty() {
        return GitInfo {
            branch: None,
            dirty: false,
            ahead: 0,
            behind: 0,
        };
    }

    let branch = git_cmd(cwd, &["symbolic-ref", "--short", "HEAD"])
        .or_else(|| git_cmd(cwd, &["rev-parse", "--short", "HEAD"]));

    if branch.is_none() {
        return GitInfo {
            branch: None,
            dirty: false,
            ahead: 0,
            behind: 0,
        };
    }

    let dirty = std::process::Command::new("git")
        .args(["-C", cwd, "diff-index", "--quiet", "HEAD", "--"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| !s.success())
        .unwrap_or(false);

    let ahead = git_cmd(cwd, &["rev-list", "--count", "@{u}..HEAD"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let behind = git_cmd(cwd, &["rev-list", "--count", "HEAD..@{u}"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    GitInfo {
        branch,
        dirty,
        ahead,
        behind,
    }
}

fn git_cmd(cwd: &str, args: &[&str]) -> Option<String> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["-C", cwd])
        .args(args)
        .stderr(std::process::Stdio::null());

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
