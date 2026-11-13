//! The dialog over the panes. One is open at a time and it takes every key
//! until it closes.

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::{
    model::{ListId, StepId, TaskId},
    tui::{account_manager::Accounts, filter_list::FilterList, input::TextInput},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    Help,
    Confirm(Confirm),
    Name(NamePrompt),
    Due(DuePrompt),
    Move(MovePicker),
    Accounts(Accounts),
}

impl Dialog {
    pub fn takes_text(&self) -> bool {
        matches!(self, Dialog::Name(_) | Dialog::Due(_) | Dialog::Move(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    pub prompt: String,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    DeleteList {
        account: usize,
        list: ListId,
    },
    DeleteTasks(Vec<(usize, ListId, TaskId)>),
    DeleteStep {
        account: usize,
        list: ListId,
        task: TaskId,
        step: StepId,
    },
    DiscardForm,
    Update,
}

/// What a typed name is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameFor {
    NewList,
    RenameList {
        account: usize,
        list: ListId,
    },
    NewTask {
        account: usize,
        list: ListId,
    },
    RenameTask {
        account: usize,
        list: ListId,
        task: TaskId,
    },
    NewStep {
        account: usize,
        list: ListId,
        task: TaskId,
    },
    RenameStep {
        account: usize,
        list: ListId,
        task: TaskId,
        step: StepId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePrompt {
    pub title: String,
    pub label: String,
    pub input: TextInput,
    pub purpose: NameFor,
}

impl NamePrompt {
    pub fn takes_text(&self) -> bool {
        true
    }

    pub fn new(title: &str, label: &str, text: &str, purpose: NameFor) -> Self {
        Self {
            title: title.to_owned(),
            label: label.to_owned(),
            input: TextInput::from(text),
            purpose,
        }
    }
}

/// What a due date is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DueFor {
    Tasks {
        account: usize,
        tasks: Vec<(ListId, TaskId)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuePrompt {
    pub input: TextInput,
    pub error: Option<String>,
    pub purpose: DueFor,
}

impl DuePrompt {
    pub fn new(purpose: DueFor) -> Self {
        Self {
            input: TextInput::default(),
            error: None,
            purpose,
        }
    }
}

/// What a key asks of the move picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveRequest {
    None,
    Cancel,
    Move(ListId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePicker {
    pub account: usize,
    pub tasks: Vec<(ListId, TaskId)>,
    pub choices: Vec<(ListId, String)>,
    pub filter: FilterList,
}

impl MovePicker {
    pub fn new(
        account: usize,
        tasks: Vec<(ListId, TaskId)>,
        choices: Vec<(ListId, String)>,
    ) -> Self {
        Self {
            account,
            tasks,
            choices,
            filter: FilterList::default(),
        }
    }

    /// The choices whose name has the filter in it.
    pub fn shown(&self) -> Vec<&(ListId, String)> {
        self.choices
            .iter()
            .filter(|(_, name)| self.filter.matches(name))
            .collect()
    }

    pub fn picked(&self) -> Option<&(ListId, String)> {
        self.shown().get(self.filter.selected).copied()
    }

    pub fn key(&mut self, key: KeyEvent) -> MoveRequest {
        match key.code {
            KeyCode::Esc => MoveRequest::Cancel,
            KeyCode::Enter => self
                .picked()
                .map_or(MoveRequest::None, |(id, _)| MoveRequest::Move(id.clone())),
            _ => {
                self.filter.key(key, self.shown().len());
                MoveRequest::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picker() -> MovePicker {
        MovePicker::new(
            0,
            vec![(ListId("a".into()), TaskId("t".into()))],
            vec![
                (ListId("a".into()), "Groceries".to_owned()),
                (ListId("b".into()), "Work".to_owned()),
            ],
        )
    }

    #[test]
    fn typing_narrows_the_move_picker() {
        let mut picker = picker();

        for c in "gro".chars() {
            picker.key(KeyEvent::from(KeyCode::Char(c)));
        }

        assert_eq!(picker.shown().len(), 1);
        assert_eq!(picker.picked().unwrap().1, "Groceries");
    }

    #[test]
    fn enter_picks_the_list_and_esc_gives_up() {
        let mut picker = picker();

        picker.key(KeyEvent::from(KeyCode::Down));
        assert_eq!(
            picker.key(KeyEvent::from(KeyCode::Enter)),
            MoveRequest::Move(ListId("b".into()))
        );
        assert_eq!(
            picker.key(KeyEvent::from(KeyCode::Esc)),
            MoveRequest::Cancel
        );
    }
}
