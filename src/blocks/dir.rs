use crate::config::BlockConfig;
use crate::style::*;

use super::RenderContext;

pub fn render(ctx: &RenderContext, _bc: &BlockConfig) -> Option<String> {
    let current = &ctx.status.workspace.current_dir;
    let project = &ctx.status.workspace.project_dir;

    if project.is_empty() {
        return None;
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let project_display = if !home.is_empty() && project.starts_with(&home) {
        format!("~{}", &project[home.len()..])
    } else {
        project.clone()
    };

    if current == project {
        Some(format!("{FG_BCYAN}{project_display}{RST}"))
    } else if current.starts_with(project.as_str()) {
        let rel = &current[project.len() + 1..];
        Some(format!("{FG_BCYAN}{project_display}/{FG_BGREEN}{rel}{RST}"))
    } else {
        let current_display = if !home.is_empty() && current.starts_with(&home) {
            format!("~{}", &current[home.len()..])
        } else {
            current.clone()
        };
        Some(format!(
            "{FG_BCYAN}{project_display}{RST} > {FG_BYELLOW}{current_display}{RST}"
        ))
    }
}
