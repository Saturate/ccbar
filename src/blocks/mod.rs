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

pub type BlockPart = (&'static str, String);

fn assemble_parts(parts: Vec<BlockPart>, bc: &BlockConfig) -> Option<String> {
    let filtered: Vec<String> = match &bc.parts {
        Some(wanted) => wanted
            .iter()
            .filter_map(|name| {
                parts
                    .iter()
                    .find(|(pn, _)| *pn == name.as_str())
                    .map(|(_, rendered)| rendered.clone())
            })
            .collect(),
        None => parts.into_iter().map(|(_, rendered)| rendered).collect(),
    };

    if filtered.is_empty() {
        None
    } else {
        Some(filtered.join(" "))
    }
}

pub fn valid_parts(block_name: &str) -> Option<&'static [&'static str]> {
    match block_name {
        "git-branch" => Some(&["icon", "branch", "dirty", "ahead-behind"]),
        "git-status" => Some(&["dirty", "ahead-behind"]),
        "model" => Some(&["name", "context-size"]),
        "context-bar" => Some(&["label", "bar", "pct"]),
        "tokens" => Some(&["input", "output"]),
        "duration" => Some(&["days", "hours", "minutes", "seconds"]),
        "rate-limit" => Some(&["5h", "7d"]),
        _ => None,
    }
}

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

    let parts = match name {
        "dir" => dir::render_parts(ctx, bc),
        "git-branch" => git::render_branch_parts(ctx, bc),
        "git-status" => git::render_status_parts(ctx, bc),
        "model" => model::render_parts(ctx, bc),
        "context-bar" => context_bar::render_parts(ctx, bc),
        "tokens" => tokens::render_parts(ctx, bc),
        "cost" => cost::render_parts(ctx, bc),
        "duration" => duration::render_parts(ctx, bc),
        "rate-limit" => rate_limit::render_parts(ctx, bc),
        _ => return None,
    };

    assemble_parts(parts, bc)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bc_with_parts(parts: Vec<&str>) -> BlockConfig {
        BlockConfig {
            parts: Some(parts.into_iter().map(String::from).collect()),
            ..BlockConfig::default()
        }
    }

    #[test]
    fn assemble_no_filter() {
        let parts = vec![("a", "A".into()), ("b", "B".into()), ("c", "C".into())];
        let bc = BlockConfig::default();
        assert_eq!(assemble_parts(parts, &bc), Some("A B C".into()));
    }

    #[test]
    fn assemble_filter_subset() {
        let parts = vec![("a", "A".into()), ("b", "B".into()), ("c", "C".into())];
        let bc = bc_with_parts(vec!["a", "c"]);
        assert_eq!(assemble_parts(parts, &bc), Some("A C".into()));
    }

    #[test]
    fn assemble_reorder() {
        let parts = vec![("a", "A".into()), ("b", "B".into()), ("c", "C".into())];
        let bc = bc_with_parts(vec!["c", "a"]);
        assert_eq!(assemble_parts(parts, &bc), Some("C A".into()));
    }

    #[test]
    fn assemble_empty_parts_returns_none() {
        let parts: Vec<BlockPart> = vec![];
        let bc = BlockConfig::default();
        assert_eq!(assemble_parts(parts, &bc), None);
    }

    #[test]
    fn assemble_filter_no_match_returns_none() {
        let parts = vec![("a", "A".into())];
        let bc = bc_with_parts(vec!["z"]);
        assert_eq!(assemble_parts(parts, &bc), None);
    }

    #[test]
    fn assemble_filter_skips_missing() {
        let parts = vec![("a", "A".into()), ("c", "C".into())];
        let bc = bc_with_parts(vec!["a", "b", "c"]);
        assert_eq!(assemble_parts(parts, &bc), Some("A C".into()));
    }

    #[test]
    fn valid_parts_known_blocks() {
        assert_eq!(valid_parts("git-branch"), Some(["icon", "branch", "dirty", "ahead-behind"].as_slice()));
        assert_eq!(valid_parts("model"), Some(["name", "context-size"].as_slice()));
        assert_eq!(valid_parts("tokens"), Some(["input", "output"].as_slice()));
    }

    #[test]
    fn valid_parts_single_part_blocks() {
        assert_eq!(valid_parts("cost"), None);
        assert_eq!(valid_parts("dir"), None);
    }

    #[test]
    fn valid_parts_duration() {
        assert_eq!(valid_parts("duration"), Some(["days", "hours", "minutes", "seconds"].as_slice()));
    }

    #[test]
    fn valid_parts_unknown_block() {
        assert_eq!(valid_parts("nonexistent"), None);
    }
}
