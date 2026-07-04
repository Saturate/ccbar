use crate::blocks;
use crate::config;
use std::fs;
use std::path::PathBuf;

pub enum Action {
    Render,
    Init,
    Setup { force: bool },
    Validate,
    Docs,
    Version,
    Help,
}

pub fn parse_args() -> Action {
    let args: Vec<String> = std::env::args().collect();
    let has_force = args.iter().any(|a| a == "--force");
    for arg in &args[1..] {
        match arg.as_str() {
            "--init" => return Action::Init,
            "--setup" => return Action::Setup { force: has_force },
            "--validate" => return Action::Validate,
            "--docs" => return Action::Docs,
            "--version" => return Action::Version,
            "--help" | "-h" => return Action::Help,
            _ => {}
        }
    }
    Action::Render
}

pub fn run_help() {
    println!(
        "\
ccbar {} — fast, configurable statusline for Claude Code

USAGE
    ccbar                   Render statusline (reads JSON from stdin)
    ccbar --setup [--force] Add statusLine to ~/.claude/settings.json
    ccbar --init            Write default config to ~/.config/ccbar/config.toml
    ccbar --validate        Check config syntax, report line/block counts
    ccbar --docs            Print config reference (for Claude Code context)
    ccbar --version         Print version
    ccbar --help, -h        Show this help

CONFIG
    {}

    To customize, read docs or just ask Claude Code:
      \"move cost to the first line\"
      \"remove the rate limit bar\"
      \"add git branch to line 2\"

DOCS
    https://github.com/Saturate/ccbar",
        env!("CARGO_PKG_VERSION"),
        config::config_path().display(),
    );
}

pub fn run_docs() {
    let path = config::config_path();
    let current = fs::read_to_string(&path).ok();

    println!("# ccbar config reference");
    println!("# Config: {}", path.display());
    println!();

    if let Some(content) = &current {
        println!("## Current config\n");
        println!("```toml");
        print!("{content}");
        println!("```");
        println!();
    } else {
        println!("## Current config: (none — using built-in defaults)\n");
    }

    print!("\
## Available blocks

Blocks are placed in `[[lines]]` arrays. Order determines render order.
Blocks with no data are hidden automatically.

### dir
Working directory with ~ abbreviation.
  segments = 0           # 0 = full path, N = last N segments
  abbreviate_home = true

### git-branch
Branch name with dirty flag and ahead/behind counts.
Includes inline status (! for dirty, arrows for ahead/behind).
  parts: icon, branch, dirty, ahead-behind (default: all)

### git-status
Standalone dirty/ahead/behind (without branch name).
Use if you want branch and status on separate lines.
  parts: dirty, ahead-behind (default: all)

### model
Model name and context window. Color by family (opus=magenta, sonnet=cyan, haiku=green).
  parts: name, context-size (default: all)

### context-bar
Visual progress bar for context window usage.
  width = 12             # bar width in characters
  thresholds = [50, 75, 90]  # color shift points
  parts: label, bar, pct (default: all)

### tokens
Input/output token counts. Auto-scales (raw, Xk, XM).
  parts: input, output (default: all)

### cost
Session cost. Default USD, supports currency conversion via frankfurter.app (ECB rates, cached 24h).
  warn = 1.0             # yellow above this
  crit = 5.0             # red above this
  currency = \"DKK\"       # any ISO 4217 code (USD, EUR, GBP, DKK, SEK, JPY, ...)

### duration
Session duration. Auto-scales from seconds to days.
  format = \"{{total_h}}h{{m:02}}m{{s:02}}s\"  # optional format string
  Tokens: {{d}} {{h}} {{m}} {{s}} {{h:02}} {{m:02}} {{s:02}} {{total_h}} {{total_m}} {{total_s}}

### rate-limit
5-hour and 7-day rate limit bars with countdowns.
  show_countdown = true
  show_bar = true
  bar_width = 8
  parts: 5h, 7d (default: all)

## Parts (sub-block composition)

Any multi-part block supports filtering and reordering via `parts`:

  [blocks.git-branch]
  parts = [\"branch\", \"dirty\"]   # drops icon and ahead-behind

  [blocks.context-bar]
  parts = [\"bar\", \"pct\"]        # drops the 'ctx' label

  [blocks.model]
  parts = [\"name\"]              # drops /1M context size

Omit `parts` to render all parts in default order.
Order in `parts` controls render order.

## Colors (per-part color overrides)

Override any part's color using a `[blocks.<name>.colors]` table:

  [blocks.duration.colors]
  days = \"cyan\"
  hours = \"magenta\"
  minutes = \"yellow\"
  seconds = \"green\"

  [blocks.git-branch.colors]
  branch = \"blue\"
  dirty = \"red\"

  [blocks.tokens.colors]
  input = \"bright-cyan\"
  output = \"bright-yellow\"

Available colors:
  red, green, yellow, blue, magenta, cyan, white
  bright-red, bright-green, bright-yellow, bright-blue
  bright-magenta, bright-cyan, bright-white
  dim

Omit `colors` to use built-in defaults.

## Separator

  [separator]
  char = \" | \"
  style = \"dim\"

## Structure

  [[lines]]              # first status line
  blocks = [\"dir\", \"git-branch\"]

  [[lines]]              # second status line
  blocks = [\"model\", \"context-bar\", \"tokens\", \"cost\", \"duration\", \"rate-limit\"]

Add as many [[lines]] as needed. Reorder or remove blocks freely.
");
}

pub fn run_init() {
    let path = config::config_path();
    if path.exists() {
        eprintln!("Config already exists: {}", path.display());
        eprintln!("Delete it first if you want to regenerate.");
        std::process::exit(1);
    }

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(&path, config::DEFAULT_TOML) {
        Ok(()) => println!("Wrote default config to {}", path.display()),
        Err(e) => {
            eprintln!("Failed to write config: {e}");
            std::process::exit(1);
        }
    }
}

pub fn run_setup(force: bool) {
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| fs::canonicalize(p).ok())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "ccbar".into());

    let settings_path = claude_settings_path();

    let mut root: serde_json::Map<String, serde_json::Value> = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| {
            eprintln!("Warning: existing {} has invalid JSON, backing up", settings_path.display());
            let backup = settings_path.with_extension("json.bak");
            let _ = fs::copy(&settings_path, &backup);
            serde_json::Map::new()
        })
    } else {
        serde_json::Map::new()
    };

    if root.contains_key("statusLine") && !force {
        eprintln!("statusLine already configured in {}", settings_path.display());
        eprintln!("Use --setup --force to overwrite.");
        std::process::exit(1);
    }

    let mut status_line = serde_json::Map::new();
    status_line.insert("type".into(), serde_json::Value::String("command".into()));
    status_line.insert("command".into(), serde_json::Value::String(binary.clone()));
    status_line.insert("refreshInterval".into(), serde_json::Value::Number(1.into()));

    root.insert("statusLine".into(), serde_json::Value::Object(status_line));

    if let Some(parent) = settings_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let json = serde_json::to_string_pretty(&root).expect("failed to serialize JSON");
    match fs::write(&settings_path, format!("{json}\n")) {
        Ok(()) => {
            println!("Configured Claude Code statusline in {}", settings_path.display());
            println!("  command: {binary}");
        }
        Err(e) => {
            eprintln!("Failed to write {}: {e}", settings_path.display());
            std::process::exit(1);
        }
    }
}

fn claude_settings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".claude").join("settings.json")
}

pub fn run_validate() {
    let path = config::config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<config::Config>(&content) {
            Ok(cfg) => {
                let block_count: usize = cfg.lines.iter().map(|l| l.blocks.len()).sum();
                let mut warnings = 0;

                for (block_name, block_cfg) in &cfg.blocks {
                    if let Some(ref parts) = block_cfg.parts {
                        match blocks::valid_parts(block_name) {
                            Some(valid) => {
                                for p in parts {
                                    if !valid.contains(&p.as_str()) {
                                        eprintln!(
                                            "Warning: block '{}' has no part '{}'. Valid: {:?}",
                                            block_name, p, valid
                                        );
                                        warnings += 1;
                                    }
                                }
                            }
                            None => {
                                eprintln!(
                                    "Warning: block '{}' is single-part, `parts` has no effect",
                                    block_name
                                );
                                warnings += 1;
                            }
                        }
                    }
                }

                println!("Valid: {} lines, {} blocks", cfg.lines.len(), block_count);
                if warnings > 0 {
                    println!("{} warning(s)", warnings);
                }
            }
            Err(e) => {
                eprintln!("Invalid config at {}: {e}", path.display());
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Cannot read {}: {e}", path.display());
            std::process::exit(1);
        }
    }
}

pub fn run_version() {
    println!("ccbar {}", env!("CARGO_PKG_VERSION"));
}
