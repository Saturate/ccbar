use crate::config::BlockConfig;
use crate::style::*;

use super::{BlockPart, RenderContext};

pub fn render_parts(ctx: &RenderContext, bc: &BlockConfig) -> Vec<BlockPart> {
    let current = &ctx.status.workspace.current_dir;
    let project = &ctx.status.workspace.project_dir;

    if project.is_empty() {
        return vec![];
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let project_display = if !home.is_empty() && project.starts_with(&home) {
        format!("~{}", &project[home.len()..])
    } else {
        project.clone()
    };

    let dir_color = bc.color_for("dir", FG_BCYAN);

    let rendered = if current == project {
        format!("{dir_color}{project_display}{RST}")
    } else if let Some(rel) = current.strip_prefix(project.as_str()).and_then(|r| r.strip_prefix('/')) {
        let sub_color = bc.color_for("subdir", FG_BGREEN);
        format!("{dir_color}{project_display}/{sub_color}{rel}{RST}")
    } else {
        let current_display = if !home.is_empty() && current.starts_with(&home) {
            format!("~{}", &current[home.len()..])
        } else {
            current.clone()
        };
        let ext_color = bc.color_for("external", FG_BYELLOW);
        format!("{dir_color}{project_display}{RST} > {ext_color}{current_display}{RST}")
    };

    vec![("dir", rendered)]
}
