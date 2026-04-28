mod blocks;
mod cli;
mod config;
mod render;
mod status;
mod style;

use blocks::{collect_git_info, RenderContext};

fn main() {
    match cli::parse_args() {
        cli::Action::Init => {
            cli::run_init();
            return;
        }
        cli::Action::Validate => {
            cli::run_validate();
            return;
        }
        cli::Action::Version => {
            cli::run_version();
            return;
        }
        cli::Action::Render => {}
    }

    let input = match read_stdin() {
        Some(s) if !s.trim().is_empty() => s,
        _ => return,
    };

    let status: status::StatusJSON = match serde_json::from_str(&input) {
        Ok(s) => s,
        Err(_) => return,
    };

    let config = config::load();
    let git = collect_git_info(&status.workspace.current_dir);

    let ctx = RenderContext {
        status: &status,
        git: &git,
        config: &config,
    };

    let lines = render::render_lines(&config, &ctx);
    for line in lines {
        println!("{line}");
    }
}

fn read_stdin() -> Option<String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok()?;
    Some(buf)
}
