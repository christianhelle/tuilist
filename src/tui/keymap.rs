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
        KeyCode::Char('o') => Action::OpenLink,
        KeyCode::Char('c') => Action::ToggleCompletedShown,
        _ => return None,
    };
    Some(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(key: KeyEvent) -> Option<Action> {
        Keymap::default().action_for(key)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn every_pane_moves_and_switches() {
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('j'))),
            Some(Action::SelectNext)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Down)),
            Some(Action::SelectNext)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('k'))),
            Some(Action::SelectPrev)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Up)),
            Some(Action::SelectPrev)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Tab)),
            Some(Action::FocusNextPane)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::BackTab)),
            Some(Action::FocusPrevPane)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('h'))),
            Some(Action::Collapse)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Left)),
            Some(Action::Collapse)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('l'))),
            Some(Action::Expand)
        );
        assert_eq!(action(KeyEvent::from(KeyCode::Right)), Some(Action::Expand));
        assert_eq!(action(KeyEvent::from(KeyCode::Enter)), Some(Action::Open));
        assert_eq!(action(KeyEvent::from(KeyCode::Home)), Some(Action::Top));
        assert_eq!(action(KeyEvent::from(KeyCode::End)), Some(Action::Bottom));
    }

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

    #[test]
    fn paging_and_the_control_keys_move_pages() {
        assert_eq!(
            action(KeyEvent::from(KeyCode::PageDown)),
            Some(Action::PageDown)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::PageUp)),
            Some(Action::PageUp)
        );
        assert_eq!(action(ctrl('f')), Some(Action::PageDown));
        assert_eq!(action(ctrl('b')), Some(Action::PageUp));
        assert_eq!(action(ctrl('d')), Some(Action::HalfPageDown));
        assert_eq!(action(ctrl('u')), Some(Action::HalfPageUp));
    }

    #[test]
    fn the_pane_and_app_keys_are_all_bound() {
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('z'))),
            Some(Action::ToggleZoom)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('b'))),
            Some(Action::ToggleLists)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('L'))),
            Some(Action::ToggleDetailsPlace)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('/'))),
            Some(Action::Search)
        );
        assert_eq!(action(KeyEvent::from(KeyCode::F(5))), Some(Action::Refresh));
        assert_eq!(action(ctrl('r')), Some(Action::Refresh));
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('M'))),
            Some(Action::ManageAccounts)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('U'))),
            Some(Action::Update)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('?'))),
            Some(Action::Help)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('q'))),
            Some(Action::Quit)
        );
        assert_eq!(action(ctrl('c')), Some(Action::Quit));
    }

    #[test]
    fn the_task_keys_are_all_bound() {
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('a'))),
            Some(Action::Add)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('x'))),
            Some(Action::ToggleComplete)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('*'))),
            Some(Action::ToggleStar)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('e'))),
            Some(Action::Edit)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('i'))),
            Some(Action::Edit)
        );
        assert_eq!(action(KeyEvent::from(KeyCode::F(2))), Some(Action::Rename));
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('t'))),
            Some(Action::DueToday)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('T'))),
            Some(Action::AskDue)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('s'))),
            Some(Action::AddStep)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('m'))),
            Some(Action::Move)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('d'))),
            Some(Action::Delete)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('D'))),
            Some(Action::Delete)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('y'))),
            Some(Action::Copy)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('o'))),
            Some(Action::OpenLink)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char('c'))),
            Some(Action::ToggleCompletedShown)
        );
        assert_eq!(
            action(KeyEvent::from(KeyCode::Char(' '))),
            Some(Action::ToggleMark)
        );
        assert_eq!(action(ctrl('a')), Some(Action::MarkAll));
        assert_eq!(
            action(KeyEvent::from(KeyCode::Esc)),
            Some(Action::ClearMarks)
        );
    }
}
