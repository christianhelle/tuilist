//! Default vim-flavoured keybindings.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::Action;

/// Turns key presses into actions, remembering a half-typed `gg`.
#[derive(Debug, Default)]
pub struct Keymap {
    pending_g: bool,
}

impl Keymap {
    pub fn action_for(&mut self, key: KeyEvent) -> Option<Action> {
        let pending_g = std::mem::take(&mut self.pending_g);
        if key.code == KeyCode::Char('g') && !key.modifiers.contains(KeyModifiers::CONTROL) {
            if pending_g {
                return Some(Action::Top);
            }
            self.pending_g = true;
            return None;
        }
        single_key(key)
    }
}

fn single_key(key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        let action = match key.code {
            KeyCode::Char('c') => Action::Quit,
            KeyCode::Char('f') => Action::PageDown,
            KeyCode::Char('b') => Action::PageUp,
            KeyCode::Char('d') => Action::HalfPageDown,
            KeyCode::Char('u') => Action::HalfPageUp,
            KeyCode::Char('a') => Action::MarkAll,
            KeyCode::Char('r') => Action::Refresh,
            _ => return None,
        };
        return Some(action);
    }
    let action = match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::SelectNext,
        KeyCode::Char('k') | KeyCode::Up => Action::SelectPrev,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::Char('G') | KeyCode::End => Action::Bottom,
        KeyCode::Home => Action::Top,
        KeyCode::Tab => Action::FocusNextPane,
        KeyCode::BackTab => Action::FocusPrevPane,
        KeyCode::Enter => Action::Open,
        KeyCode::Char('l') | KeyCode::Right => Action::Expand,
        KeyCode::Char('h') | KeyCode::Left => Action::Collapse,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('z') => Action::ToggleZoom,
        KeyCode::Char('b') => Action::ToggleLists,
        KeyCode::Char('L') => Action::ToggleDetailsPlace,
        KeyCode::Char(' ') => Action::ToggleMark,
        KeyCode::Esc => Action::ClearMarks,
        KeyCode::Char('d') | KeyCode::Char('D') => Action::Delete,
        KeyCode::F(2) => Action::Rename,
        KeyCode::F(5) => Action::Refresh,
        KeyCode::Char('?') => Action::Help,
        KeyCode::Char('/') => Action::Search,
        KeyCode::Char('U') => Action::Update,
        KeyCode::Char('M') => Action::ManageAccounts,
        KeyCode::Char('a') => Action::Add,
        KeyCode::Char('x') => Action::ToggleComplete,
        KeyCode::Char('*') => Action::ToggleStar,
        KeyCode::Char('e') | KeyCode::Char('i') => Action::Edit,
        KeyCode::Char('t') => Action::DueToday,
        KeyCode::Char('T') => Action::AskDue,
        KeyCode::Char('s') => Action::AddStep,
        KeyCode::Char('m') => Action::Move,
        KeyCode::Char('y') => Action::Copy,
        KeyCode::Char('c') => Action::ToggleCompletedShown,
        _ => return None,
    };
    Some(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_gs_jump_to_the_top() {
        let mut keymap = Keymap::default();

        assert_eq!(keymap.action_for(KeyEvent::from(KeyCode::Char('g'))), None);
        assert_eq!(
            keymap.action_for(KeyEvent::from(KeyCode::Char('g'))),
            Some(Action::Top)
        );
        assert_eq!(
            keymap.action_for(KeyEvent::from(KeyCode::Char('g'))),
            None,
            "the count starts again"
        );
    }
}
