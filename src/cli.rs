use crate::config;
use std::fs;
use std::path::PathBuf;

pub enum Action {
    Render,
    Init,
    Setup { force: bool },
    Validate,
    Version,
}

pub fn parse_args() -> Action {
    let args: Vec<String> = std::env::args().collect();
    let has_force = args.iter().any(|a| a == "--force");
    for arg in &args[1..] {
        match arg.as_str() {
            "--init" => return Action::Init,
            "--setup" => return Action::Setup { force: has_force },
            "--validate" => return Action::Validate,
            "--version" => return Action::Version,
            _ => {}
        }
    }
    Action::Render
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
                println!("Valid: {} lines, {} blocks", cfg.lines.len(), block_count);
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
