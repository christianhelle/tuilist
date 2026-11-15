//! Tasks: the things Microsoft To Do is about.

use std::cmp::Ordering;

use jiff::{Timestamp, civil::Date};

use crate::model::ListId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    NotStarted,
    InProgress,
    Completed,
    WaitingOnOthers,
    Deferred,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::NotStarted => "notStarted",
            Status::InProgress => "inProgress",
            Status::Completed => "completed",
            Status::WaitingOnOthers => "waitingOnOthers",
            Status::Deferred => "deferred",
        }
    }

    /// Reads the value Graph uses, falling back to `notStarted` for anything
    /// this version does not know.
    pub fn from_graph(value: &str) -> Self {
        match value {
            "completed" => Status::Completed,
            "inProgress" => Status::InProgress,
            "waitingOnOthers" => Status::WaitingOnOthers,
            "deferred" => Status::Deferred,
            _ => Status::NotStarted,
        }
    }

    /// Every status except `completed` means the task is still open.
    pub fn is_open(self) -> bool {
        self != Status::Completed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Importance {
    Low,
    Normal,
    High,
}

impl Importance {
    pub fn as_str(self) -> &'static str {
        match self {
            Importance::Low => "low",
            Importance::Normal => "normal",
            Importance::High => "high",
        }
    }

    pub fn from_graph(value: &str) -> Self {
        match value {
            "low" => Importance::Low,
            "high" => Importance::High,
            _ => Importance::Normal,
        }
    }

    pub fn is_starred(self) -> bool {
        self == Importance::High
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub list: ListId,
    pub title: String,
    pub status: Status,
    pub importance: Importance,
    pub due: Option<Date>,
    /// `None` when the reminder is off.
    pub reminder: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub modified_at: Timestamp,
    pub notes: String,
    pub notes_html: bool,
    pub recurring: bool,
    pub has_attachments: bool,
    /// From the store, 0/0 until steps have been fetched.
    pub steps_done: u32,
    pub steps_total: u32,
    pub steps_loaded: bool,
}

/// What a create or an edit sends. `None` leaves a field alone; `Some(None)`
/// clears it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskDraft {
    pub title: Option<String>,
    pub starred: Option<bool>,
    pub completed: Option<bool>,
    pub due: Option<Option<Date>>,
    pub reminder: Option<Option<Timestamp>>,
    pub notes: Option<String>,
}

impl TaskDraft {
    pub fn is_empty(&self) -> bool {
        self == &TaskDraft::default()
    }
}

/// Open tasks newest first, then completed tasks newest completion first.
pub fn sort_tasks(tasks: &mut [Task]) {
    tasks.sort_by(compare_tasks);
}

fn compare_tasks(a: &Task, b: &Task) -> Ordering {
    a.status
        .is_open()
        .cmp(&b.status.is_open())
        .reverse()
        .then_with(|| match (a.status.is_open(), b.status.is_open()) {
            (true, true) => b.created_at.cmp(&a.created_at),
            (false, false) => b.completed_at.cmp(&a.completed_at),
            _ => Ordering::Equal,
        })
        .then_with(|| a.id.cmp(&b.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, status: Status, created: i64, completed: Option<i64>) -> Task {
        Task {
            id: TaskId(id.to_owned()),
            list: ListId("list".to_owned()),
            title: id.to_owned(),
            status,
            importance: Importance::Normal,
            due: None,
            reminder: None,
            completed_at: completed.map(|seconds| Timestamp::from_second(seconds).unwrap()),
            created_at: Timestamp::from_second(created).unwrap(),
            modified_at: Timestamp::from_second(created).unwrap(),
            notes: String::new(),
            notes_html: false,
            recurring: false,
            has_attachments: false,
            steps_done: 0,
            steps_total: 0,
            steps_loaded: false,
        }
    }

    #[test]
    fn tasks_are_newest_first_with_completed_last() {
        let mut tasks = vec![
            task("old", Status::NotStarted, 10, None),
            task("done-old", Status::Completed, 5, Some(20)),
            task("new", Status::NotStarted, 30, None),
            task("done-new", Status::Completed, 1, Some(40)),
        ];

        sort_tasks(&mut tasks);

        let ids: Vec<&str> = tasks.iter().map(|task| task.id.0.as_str()).collect();
        assert_eq!(ids, ["new", "old", "done-new", "done-old"]);
    }
}
