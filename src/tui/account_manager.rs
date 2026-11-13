//! The accounts dialog: the addresses signed in, for looking at and removing.

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::AccountView;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Accounts {
    pub addresses: Vec<String>,
    pub selected: usize,
}

impl Accounts {
    pub fn of(accounts: &[AccountView]) -> Self {
        Self {
            addresses: accounts
                .iter()
                .map(|account| account.address.clone())
                .collect(),
            selected: 0,
        }
    }

    pub fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.step(1),
            KeyCode::Up | KeyCode::Char('k') => self.step(-1),
            _ => {}
        }
    }

    fn step(&mut self, delta: isize) {
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.addresses.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selection_stays_on_an_address() {
        let mut accounts = Accounts {
            addresses: vec!["ada@outlook.com".to_owned(), "ada@work.example".to_owned()],
            selected: 0,
        };

        accounts.key(KeyEvent::from(KeyCode::Up));
        assert_eq!(accounts.selected, 0);
        accounts.key(KeyEvent::from(KeyCode::Down));
        accounts.key(KeyEvent::from(KeyCode::Down));
        assert_eq!(accounts.selected, 1);
    }
}
