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
