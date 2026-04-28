use crate::config;
use std::fs;

pub enum Action {
    Render,
    Init,
    Validate,
    Version,
}

pub fn parse_args() -> Action {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        match arg.as_str() {
            "--init" => return Action::Init,
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
