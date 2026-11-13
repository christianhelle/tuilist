//! The search line and its live results.

use crate::tui::input::TextInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Typing,
    Results,
}

/// What is typed into the search line, and whether Enter has moved into the
/// results.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    pub input: TextInput,
    step: Option<Step>,
}

impl Search {
    pub fn typing(&self) -> bool {
        !matches!(self.step, Some(Step::Results))
    }

    pub fn finish(&mut self) {
        self.step = Some(Step::Results);
    }

    pub fn query(&self) -> &str {
        self.input.text()
    }

    pub fn step(&self) -> Step {
        self.step.unwrap_or(Step::Typing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};

    #[test]
    fn a_new_search_is_being_typed_and_enter_moves_to_the_results() {
        let mut search = Search::default();
        assert!(search.typing());

        search.input.key(KeyEvent::from(KeyCode::Char('m')));
        search.finish();

        assert!(!search.typing());
        assert_eq!(search.query(), "m");
        assert_eq!(search.step(), Step::Results);
    }
}
