//! The place to start from next time.

use std::path::PathBuf;

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::tui::app::{AppState, Pane, Smart, View};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub focus: String,
    pub list: Option<String>,
    pub task: usize,
    pub zoomed: bool,
    pub lists_hidden: bool,
    pub details_below: bool,
    pub completed_hidden: bool,
    pub collapsed_groups: Vec<String>,
}

impl Session {
    pub fn of(app: &AppState) -> Self {
        Self {
            focus: match app.focus {
                Pane::Lists => "lists".to_owned(),
                Pane::Tasks => "tasks".to_owned(),
                Pane::Details => "details".to_owned(),
            },
            list: match &app.view {
                View::Smart(smart) => Some(smart.name().to_owned()),
                View::List { id, .. } => Some(id.0.clone()),
            },
            task: app.selected_task,
            zoomed: app.zoomed,
            lists_hidden: app.lists_hidden,
            details_below: app.details_below,
            completed_hidden: app.completed_hidden,
            collapsed_groups: app.collapsed_groups.iter().cloned().collect(),
        }
    }

    pub fn apply(&self, app: &mut AppState) {
        app.focus = match self.focus.as_str() {
            "tasks" => Pane::Tasks,
            "details" => Pane::Details,
            _ => Pane::Lists,
        };
        app.zoomed = self.zoomed;
        app.lists_hidden = self.lists_hidden;
        app.details_below = self.details_below;
        app.completed_hidden = self.completed_hidden;
        app.collapsed_groups = self.collapsed_groups.iter().cloned().collect();
        if let Some(list) = &self.list
            && let Some(index) = app.rows.iter().position(|row| match row {
                crate::tui::app::ListRow::Smart(smart) => Some(*smart) == Smart::from_name(list),
                crate::tui::app::ListRow::List { list: row, .. } => &row.id.0 == list,
                crate::tui::app::ListRow::Account(_) => false,
            })
        {
            app.select_list(index);
        }
        app.selected_task = self.task;
    }

    pub fn path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "tuilist").map(|dirs| dirs.cache_dir().join("session.json"))
    }

    pub fn load() -> Option<Self> {
        let path = Self::path()?;
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::AppState;

    #[test]
    fn the_place_is_read_back_as_it_was_left() {
        let mut app = AppState {
            focus: Pane::Tasks,
            zoomed: true,
            completed_hidden: true,
            ..AppState::default()
        };
        app.collapsed_groups.insert("completed".to_owned());

        let session = Session::of(&app);
        let mut restored = AppState::default();
        session.apply(&mut restored);

        assert_eq!(restored.focus, Pane::Tasks);
        assert!(restored.zoomed);
        assert!(restored.completed_hidden);
        assert!(restored.collapsed_groups.contains("completed"));
    }
}
