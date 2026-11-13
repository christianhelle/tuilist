//! What the details pane shows: the selected task's fields, its steps and its
//! notes in a read-only vim viewer.

use std::collections::HashMap;

use jiff::tz::TimeZone;

use crate::{
    model::{Step, StepId, Task},
    tui::{list_edit, viewer::Viewer},
};

const NOTES_WIDTH: usize = 72;

#[derive(Debug, Default)]
pub struct Details {
    pub viewer: Viewer,
    steps: HashMap<usize, (StepId, bool, String)>,
}

impl Details {
    pub fn set(&mut self, task: &Task, steps: Option<&[Step]>, list_name: &str, zone: TimeZone) {
        let mut lines = Vec::new();
        lines.push(task.title.clone());
        let mut status = if task.status.is_open() {
            "[ ] Not completed".to_owned()
        } else {
            let when = task.completed_at.map_or_else(String::new, |at| {
                format!(
                    " {}",
                    list_edit::format_date(at.to_zoned(zone.clone()).date())
                )
            });
            format!("[x] Completed{when}")
        };
        if task.importance.is_starred() {
            status.push_str("  ★ Starred");
        }
        lines.push(status);
        if let Some(due) = task.due {
            lines.push(format!("Due: {}", list_edit::format_date(due)));
        }
        if let Some(reminder) = task.reminder {
            let local = reminder.to_zoned(zone.clone());
            lines.push(format!("Remind: {}", local.strftime("%a %-d %b, %H:%M")));
        }
        if task.recurring {
            lines.push("Repeats".to_owned());
        }
        if !list_name.is_empty() {
            lines.push(format!("List: {list_name}"));
        }
        if let Some(steps) = steps {
            if !steps.is_empty() {
                lines.push(String::new());
                lines.push("Steps".to_owned());
                for step in steps {
                    self.steps.insert(
                        lines.len(),
                        (step.id.clone(), step.checked, step.name.clone()),
                    );
                    lines.push(format!(
                        "{} {}",
                        if step.checked { "[x]" } else { "[ ]" },
                        step.name
                    ));
                }
            }
        } else if !task.steps_loaded {
            lines.push(String::new());
            lines.push("Steps".to_owned());
            lines.push("Loading steps…".to_owned());
        }
        if !task.notes.trim().is_empty() {
            lines.push(String::new());
            lines.push("Notes".to_owned());
            lines.extend(wrapped(&task.notes, NOTES_WIDTH));
        }
        lines.push(String::new());
        lines.push(format!(
            "Created {} · Changed {}",
            list_edit::format_timestamp_date(task.created_at, &zone),
            list_edit::format_timestamp_date(task.modified_at, &zone)
        ));
        self.viewer.show(&lines);
    }

    pub fn clear(&mut self) {
        self.steps.clear();
        self.viewer.show(&[]);
    }

    /// The step shown on `row`, and whether it is checked.
    pub fn step_at(&self, row: usize) -> Option<(StepId, bool)> {
        self.steps
            .get(&row)
            .map(|(id, checked, _)| (id.clone(), *checked))
    }

    /// The name of the step `id`.
    pub fn step_name(&self, id: &StepId) -> Option<String> {
        self.steps
            .values()
            .find(|(step, _, _)| step == id)
            .map(|(_, _, name)| name.clone())
    }

    pub fn lines(&self) -> &[String] {
        self.viewer.lines()
    }

    pub fn scroll(&self, height: usize) -> usize {
        self.viewer.scroll(height)
    }

    pub fn follow(&mut self, height: usize) {
        self.viewer.follow(height);
    }

    pub fn number_width(&self) -> usize {
        self.viewer.number_width()
    }
}

fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if line.is_empty() {
                line.push_str(word);
            } else if line.chars().count() + 1 + word.chars().count() <= width {
                line.push(' ');
                line.push_str(word);
            } else {
                lines.push(std::mem::take(&mut line));
                line.push_str(word);
            }
        }
        lines.push(line);
    }
    lines
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
            importance: Importance::High,
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

    #[test]
    fn a_task_shows_its_title_state_due_and_notes() {
        let mut details = Details::default();

        details.set(&task(), Some(&[]), "", TimeZone::UTC);

        let lines = details.lines();
        assert_eq!(lines[0], "Milk");
        assert_eq!(lines[1], "[ ] Not completed  ★ Starred");
        assert_eq!(lines[2], "Due: Fri 20 Nov 2026");
        assert!(lines.contains(&"Notes".to_owned()));
        assert!(lines.contains(&"Semi-skimmed".to_owned()));
    }

    #[test]
    fn step_rows_are_remembered_for_the_keys_to_act_on() {
        let mut details = Details::default();
        let step = Step {
            id: StepId("s1".to_owned()),
            task: TaskId("t1".to_owned()),
            name: "Check the fridge".to_owned(),
            checked: true,
            created_at: Timestamp::from_second(1_700_000_000).unwrap(),
        };

        details.set(&task(), Some(&[step]), "", TimeZone::UTC);

        let row = details
            .lines()
            .iter()
            .position(|line| line == "[x] Check the fridge")
            .unwrap();
        assert_eq!(details.step_at(row), Some((StepId("s1".to_owned()), true)));
        assert_eq!(details.step_at(0), None);
    }

    #[test]
    fn steps_not_yet_loaded_say_so() {
        let mut details = Details::default();
        let mut task = task();
        task.steps_loaded = false;

        details.set(&task, None, "Groceries", TimeZone::UTC);

        assert!(details.lines().contains(&"Loading steps…".to_owned()));
        assert!(details.lines().contains(&"List: Groceries".to_owned()));
    }
}
