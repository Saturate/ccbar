use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct StatusJSON {
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub context_window: ContextWindow,
    pub cost: Option<Cost>,
    pub rate_limits: Option<RateLimits>,
}

#[derive(Deserialize, Default)]
pub struct Workspace {
    #[serde(default)]
    pub current_dir: String,
    #[serde(default)]
    pub project_dir: String,
}

#[derive(Deserialize, Default)]
pub struct Model {
    #[serde(default)]
    pub id: String,
    pub display_name: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ContextWindow {
    #[serde(default)]
    pub context_window_size: u64,
    pub current_usage: Option<Usage>,
    #[serde(default)]
    pub total_input_tokens: u64,
    #[serde(default)]
    pub total_output_tokens: u64,
}

#[derive(Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Deserialize)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
}

#[derive(Deserialize)]
pub struct RateLimits {
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
}

#[derive(Deserialize)]
pub struct RateWindow {
    pub used_percentage: Option<f64>,
    pub resets_at: Option<i64>,
}

impl StatusJSON {
    pub fn context_tokens(&self) -> u64 {
        self.context_window
            .current_usage
            .as_ref()
            .map(|u| u.input_tokens + u.cache_creation_input_tokens + u.cache_read_input_tokens)
            .unwrap_or(0)
    }

    pub fn context_pct(&self) -> u64 {
        let size = self.context_window.context_window_size;
        if size == 0 {
            return 0;
        }
        self.context_tokens() * 100 / size
    }

    pub fn model_short(&self) -> String {
        let name = self
            .model
            .display_name
            .as_deref()
            .unwrap_or(&self.model.id);
        name.split('(').next().unwrap_or(name).trim().to_lowercase()
    }

    pub fn context_label(&self) -> &str {
        if self.context_window.context_window_size >= 1_000_000 {
            "1M"
        } else {
            "200k"
        }
    }

    pub fn total_cost(&self) -> f64 {
        self.cost
            .as_ref()
            .and_then(|c| c.total_cost_usd)
            .unwrap_or(0.0)
    }

    pub fn duration_ms(&self) -> u64 {
        self.cost
            .as_ref()
            .and_then(|c| c.total_duration_ms)
            .unwrap_or(0)
    }
}
