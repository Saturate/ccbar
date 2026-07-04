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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    pub currency: Option<String>,
    pub parts: Option<Vec<String>>,
    pub colors: Option<HashMap<String, String>>,
}

impl BlockConfig {
    pub fn color_for(&self, part: &str, default: &'static str) -> &str {
        self.colors
            .as_ref()
            .and_then(|c| c.get(part))
            .map(|name| crate::style::resolve_color(name))
            .unwrap_or(default)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_toml_parses() {
        let cfg: Config = toml::from_str(DEFAULT_TOML).unwrap();
        assert_eq!(cfg.lines.len(), 2);
        assert!(cfg.lines[0].blocks.contains(&"dir".to_string()));
        assert!(cfg.lines[1].blocks.contains(&"cost".to_string()));
    }

    #[test]
    fn empty_config_uses_defaults() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.lines.len(), 2);
        assert_eq!(cfg.separator.char, " │ ");
    }

    #[test]
    fn currency_field_parses() {
        let toml = r#"
[[lines]]
blocks = ["cost"]

[blocks.cost]
currency = "DKK"
warn = 7.0
crit = 35.0
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let cost_bc = cfg.blocks.get("cost").unwrap();
        assert_eq!(cost_bc.currency.as_deref(), Some("DKK"));
        assert_eq!(cost_bc.warn, Some(7.0));
        assert_eq!(cost_bc.crit, Some(35.0));
    }

    #[test]
    fn custom_separator() {
        let toml = r#"
[separator]
char = " | "
style = "bold"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.separator.char, " | ");
        assert_eq!(cfg.separator.style, "bold");
    }

    #[test]
    fn parts_field_parses() {
        let toml = r#"
[[lines]]
blocks = ["git-branch"]

[blocks.git-branch]
parts = ["branch", "dirty"]
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let bc = cfg.blocks.get("git-branch").unwrap();
        assert_eq!(
            bc.parts.as_deref(),
            Some(vec!["branch".to_string(), "dirty".to_string()].as_slice())
        );
    }

    #[test]
    fn colors_field_parses() {
        let toml = r#"
[[lines]]
blocks = ["duration"]

[blocks.duration.colors]
hours = "bright-cyan"
minutes = "yellow"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let bc = cfg.blocks.get("duration").unwrap();
        let colors = bc.colors.as_ref().unwrap();
        assert_eq!(colors.get("hours").unwrap(), "bright-cyan");
        assert_eq!(colors.get("minutes").unwrap(), "yellow");
    }

    #[test]
    fn color_for_with_override() {
        let toml = r#"
[[lines]]
blocks = ["git-branch"]

[blocks.git-branch.colors]
branch = "red"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let bc = cfg.blocks.get("git-branch").unwrap();
        assert_eq!(bc.color_for("branch", "\x1b[95m"), "\x1b[31m");
        assert_eq!(bc.color_for("dirty", "\x1b[33m"), "\x1b[33m");
    }

    #[test]
    fn single_line_config() {
        let toml = r#"
[[lines]]
blocks = ["model", "cost"]
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.lines.len(), 1);
        assert_eq!(cfg.lines[0].blocks, vec!["model", "cost"]);
    }
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
