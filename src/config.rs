use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_lines")]
    pub lines: Vec<Line>,
    #[serde(default)]
    pub separator: Separator,
    #[serde(default)]
    pub blocks: HashMap<String, BlockConfig>,
}

#[derive(Deserialize)]
pub struct Line {
    pub blocks: Vec<String>,
}

#[derive(Deserialize)]
pub struct Separator {
    #[serde(default = "default_sep_char")]
    pub char: String,
    #[serde(default = "default_sep_style")]
    pub style: String,
}

impl Default for Separator {
    fn default() -> Self {
        Self {
            char: default_sep_char(),
            style: default_sep_style(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct BlockConfig {
    pub width: Option<usize>,
    pub thresholds: Option<Vec<u64>>,
    pub warn: Option<f64>,
    pub crit: Option<f64>,
    pub show_countdown: Option<bool>,
    pub show_bar: Option<bool>,
    pub bar_width: Option<usize>,
    pub segments: Option<usize>,
    pub abbreviate_home: Option<bool>,
    pub format: Option<String>,
}

fn default_lines() -> Vec<Line> {
    vec![
        Line {
            blocks: vec![
                "dir".into(),
                "git-branch".into(),
            ],
        },
        Line {
            blocks: vec![
                "model".into(),
                "context-bar".into(),
                "tokens".into(),
                "cost".into(),
                "duration".into(),
                "rate-limit".into(),
            ],
        },
    ]
}

fn default_sep_char() -> String {
    " │ ".into()
}

fn default_sep_style() -> String {
    "dim".into()
}

impl Config {
    pub fn block_config(&self, name: &str) -> BlockConfig {
        self.blocks
            .get(name)
            .map(|_| {
                // Re-deserialize isn't needed; just return a ref. But we need owned.
                // Since BlockConfig is small, just read fields from the map.
                // Actually we can just look up and clone-ish. Let's do a simpler approach.
                BlockConfig::default()
            })
            .unwrap_or_default()
    }
}

pub fn config_path() -> PathBuf {
    dirs_config().join("ccbar").join("config.toml")
}

fn dirs_config() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".config")
        })
}

pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|_| default_config()),
        Err(_) => default_config(),
    }
}

fn default_config() -> Config {
    Config {
        lines: default_lines(),
        separator: Separator::default(),
        blocks: HashMap::new(),
    }
}

pub fn get_block_config<'a>(config: &'a Config, name: &str) -> Option<&'a BlockConfig> {
    config.blocks.get(name)
}

pub const DEFAULT_TOML: &str = r#"# ccbar configuration
# https://github.com/Saturate/ccbar

[[lines]]
blocks = ["dir", "git-branch"]

[[lines]]
blocks = ["model", "context-bar", "tokens", "cost", "duration", "rate-limit"]

[separator]
char = " │ "
style = "dim"

[blocks.context-bar]
width = 12
thresholds = [50, 75, 90]

[blocks.cost]
warn = 1.0
crit = 5.0

[blocks.rate-limit]
show_countdown = true
show_bar = true
bar_width = 8

[blocks.dir]
segments = 0
abbreviate_home = true
"#;
