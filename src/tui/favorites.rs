//! Favorites: lists picked to be shown again above the smart views.

use crate::tui::app::{AppState, Effect};

impl AppState {
    /// `*` in the lists pane: adds the selected list to the favorites, or
    /// takes it out if it is in them already.
    pub(crate) fn toggle_favorite(&mut self) -> Effect {
        let Some((_, list)) = self.selected_list() else {
            return Effect::None;
        };
        match self.favorites.iter().position(|id| id == &list.id) {
            Some(index) => {
                self.favorites.remove(index);
            }
            None => self.favorites.push(list.id),
        }
        self.rebuild_rows();
        Effect::None
    }
}
