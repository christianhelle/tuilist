//! The form that edits a task: its title, due date, reminder, star and notes.

use jiff::{civil::Date, tz::TimeZone};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    model::{StepId, Task, TaskDraft},
    tui::{
        editor::Editor,
        input::TextInput,
        list_edit::{format_reminder, parse_due, parse_reminder},
        screen::Cursor,
        vim::{self, Outcome, Register, VimMode},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    Due,
    Remind,
    Starred,
    Notes,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Field::Title => Field::Due,
            Field::Due => Field::Remind,
            Field::Remind => Field::Starred,
            Field::Starred => Field::Notes,
            Field::Notes => Field::Title,
        }
    }

    fn previous(self) -> Self {
        match self {
            Field::Title => Field::Notes,
            Field::Due => Field::Title,
            Field::Remind => Field::Due,
            Field::Starred => Field::Remind,
            Field::Notes => Field::Starred,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    None,
    Copy(String),
    Save(TaskDraft),
    Cancel,
    AskDiscard,
}

#[derive(Debug)]
pub struct TaskForm {
    original: Task,
    pub title: TextInput,
    pub due: TextInput,
    pub remind: TextInput,
    pub starred: bool,
    pub notes: Editor,
    pub field: Field,
    pub due_error: Option<String>,
    pub remind_error: Option<String>,
    zone: TimeZone,
    today: Date,
}

impl TaskForm {
    pub fn open(task: Task, zone: TimeZone, today: Date) -> Self {
        let due_text = task
            .due
            .map(|due| due.strftime("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let remind_text = task
            .reminder
            .map(|reminder| format_reminder(reminder, &zone))
            .unwrap_or_default();
        let mut notes = Editor::default();
        notes.set_text(&task.notes);
        notes.vim.mode = VimMode::Normal;
        Self {
            title: TextInput::from(task.title.as_str()),
            due: TextInput::from(due_text.as_str()),
            remind: TextInput::from(remind_text.as_str()),
            starred: task.importance.is_starred(),
            original: task,
            notes,
            field: Field::Title,
            due_error: None,
            remind_error: None,
            zone,
            today,
        }
    }

    pub fn field(&self) -> Field {
        self.field
    }

    pub fn cursor(&self) -> Cursor {
        match self.field {
            Field::Notes if self.notes.vim.mode != VimMode::Insert => Cursor::Block,
            _ => Cursor::Bar,
        }
    }

    pub fn notes_lines(&self) -> &[String] {
        self.notes.lines()
    }

    pub fn notes_cursor(&self) -> (usize, usize) {
        self.notes.cursor()
    }

    pub fn notes_scroll(&self, height: usize) -> usize {
        self.notes.scroll(height)
    }

    pub fn key(&mut self, key: KeyEvent, page: usize, register: &mut Register) -> Request {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('s') {
            return self.save();
        }
        match key.code {
            KeyCode::Esc => {
                if self.field == Field::Notes && self.notes.vim.mode == VimMode::Insert {
                    vim::key(&mut self.notes, key, page, register);
                    return Request::None;
                }
                return if self.has_changes() {
                    Request::AskDiscard
                } else {
                    Request::Cancel
                };
            }
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Enter => {
                self.field = if key.code == KeyCode::BackTab {
                    self.field.previous()
                } else {
                    self.field.next()
                };
                return Request::None;
            }
            KeyCode::Down if self.field != Field::Notes => {
                self.field = self.field.next();
                return Request::None;
            }
            KeyCode::Up if self.field != Field::Notes => {
                self.field = self.field.previous();
                return Request::None;
            }
            _ => {}
        }
        match self.field {
            Field::Title => {
                self.title.key(key);
            }
            Field::Due => {
                self.due.key(key);
                self.due_error = None;
            }
            Field::Remind => {
                self.remind.key(key);
                self.remind_error = None;
            }
            Field::Starred => {
                if key.code == KeyCode::Char(' ') {
                    self.starred = !self.starred;
                }
            }
            Field::Notes => {
                if let Outcome::Yanked(text) = vim::key(&mut self.notes, key, page, register) {
                    return Request::Copy(text);
                }
            }
        }
        Request::None
    }

    fn save(&mut self) -> Request {
        let due = match parse_due(self.due.text(), self.today) {
            Ok(due) => due,
            Err(error) => {
                self.due_error = Some(error);
                return Request::None;
            }
        };
        let reminder = match parse_reminder(self.remind.text(), &self.zone) {
            Ok(reminder) => reminder,
            Err(error) => {
                self.remind_error = Some(error);
                return Request::None;
            }
        };
        let mut draft = TaskDraft::default();
        let title = self.title.text().trim();
        if title != self.original.title {
            draft.title = Some(title.to_owned());
        }
        if due != self.original.due {
            draft.due = Some(due);
        }
        if reminder != self.original.reminder {
            draft.reminder = Some(reminder);
        }
        if self.starred != self.original.importance.is_starred() {
            draft.starred = Some(self.starred);
        }
        if self.notes.text() != self.original.notes {
            draft.notes = Some(self.notes.text());
        }
        if draft.is_empty() {
            Request::Cancel
        } else {
            Request::Save(draft)
        }
    }

    fn has_changes(&self) -> bool {
        self.title.text() != self.original.title
            || self.due.text() != formatted_due(&self.original)
            || self.remind.text() != formatted_reminder(&self.original, &self.zone)
            || self.starred != self.original.importance.is_starred()
            || self.notes.text() != self.original.notes
    }

    /// Unused now, kept for callers that need the step a form edits.
    pub fn step(&self) -> Option<StepId> {
        None
    }
}

fn formatted_due(task: &Task) -> String {
    task.due
        .map(|due| due.strftime("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn formatted_reminder(task: &Task, zone: &TimeZone) -> String {
    task.reminder
        .map(|reminder| format_reminder(reminder, zone))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Importance, ListId, Status, TaskId};
    use jiff::{Timestamp, civil::date};

    fn task() -> Task {
        let at = Timestamp::from_second(1_700_000_000).unwrap();
        Task {
            id: TaskId("t1".to_owned()),
            list: ListId("l1".to_owned()),
            title: "Milk".to_owned(),
            status: Status::NotStarted,
            importance: Importance::Normal,
            due: Some(date(2026, 11, 20)),
            reminder: None,
            completed_at: None,
            created_at: at,
            modified_at: at,
            notes: "Semi-skimmed".to_owned(),
            notes_html: false,
            recurring: false,
            has_attachments: false,
            steps_done: 0,
            steps_total: 0,
            steps_loaded: true,
        }
    }

    fn form() -> TaskForm {
        TaskForm::open(task(), TimeZone::UTC, date(2026, 11, 13))
    }

    fn type_text(form: &mut TaskForm, text: &str) -> Request {
        let mut request = Request::None;
        for c in text.chars() {
            let code = match c {
                '\n' => KeyCode::Enter,
                c => KeyCode::Char(c),
            };
            request = form.key(KeyEvent::from(code), 10, &mut Register::default());
        }
        request
    }

    #[test]
    fn the_form_starts_on_the_title_with_the_tasks_values() {
        let form = form();

        assert_eq!(form.field(), Field::Title);
        assert_eq!(form.title.text(), "Milk");
        assert_eq!(form.due.text(), "2026-11-20");
        assert_eq!(form.notes.text(), "Semi-skimmed");
    }

    #[test]
    fn ctrl_s_saves_only_what_changed() {
        let mut form = form();
        form.field = Field::Due;
        form.due = TextInput::from("+2");
        form.key(
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            10,
            &mut Register::default(),
        );

        let request = form.save();

        assert_eq!(
            request,
            Request::Save(TaskDraft {
                due: Some(Some(date(2026, 11, 15))),
                ..TaskDraft::default()
            })
        );
    }

    #[test]
    fn a_bad_due_date_stays_in_the_form_and_says_why() {
        let mut form = form();
        form.field = Field::Due;
        form.due = TextInput::from("next week");

        let request = form.save();

        assert_eq!(request, Request::None);
        assert!(form.due_error.is_some());
    }

    #[test]
    fn esc_asks_before_losing_changes() {
        let mut form = form();
        type_text(&mut form, "!");

        assert_eq!(
            form.key(KeyEvent::from(KeyCode::Esc), 10, &mut Register::default()),
            Request::AskDiscard
        );
    }

    #[test]
    fn a_form_with_no_changes_closes_without_asking() {
        let mut form = form();

        assert_eq!(
            form.key(KeyEvent::from(KeyCode::Esc), 10, &mut Register::default()),
            Request::Cancel
        );
    }

    #[test]
    fn space_stars_the_task_on_the_starred_field() {
        let mut form = form();
        form.field = Field::Starred;

        form.key(
            KeyEvent::from(KeyCode::Char(' ')),
            10,
            &mut Register::default(),
        );

        assert!(form.starred);
    }
}
