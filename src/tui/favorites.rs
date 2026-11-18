//! Favorites: lists picked to be shown again above the smart views.

use crate::{
    model::TaskList,
    tui::app::{AppState, Effect},
};

impl AppState {
    /// The name a favorite shows. When another account has a favorite with
    /// the same name, it also says which account it is from.
    pub fn favorite_label(&self, account: usize, list: &TaskList) -> String {
        let clashes = self.favorite_rows().iter().any(|(other, favorite)| {
            *other != account && favorite.name.eq_ignore_ascii_case(&list.name)
        });
        match self.accounts.get(account) {
            Some(view) if clashes => format!("{} - {}", list.name, view.address),
            _ => list.name.clone(),
        }
    }

    /// `*` in the lists pane: adds the selected list to the favorites, or
    /// takes it out if it is in them already.
    pub(crate) fn toggle_favorite(&mut self) -> Effect {
        let Some((_, list)) = self.selected_list() else {
            return Effect::None;
        };
        match self.favorites.iter().position(|id| id == &list.id) {
            Some(index) => {
                self.favorites.remove(index);
                self.status_info(format!("Removed {} from Favorites", list.name));
            }
            None => {
                self.favorites.push(list.id);
                self.status_info(format!("Added {} to Favorites", list.name));
            }
        }
        self.rebuild_rows();
        Effect::None
    }
}
