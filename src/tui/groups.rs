//! The headings tasks sit under: their list, the planned date bucket, or
//! `Completed`. A heading folds with `h` and unfolds with `l` or `Enter`.

use jiff::{Span, civil::Date};

use crate::{model::Task, tui::app::View};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Group {
    /// No heading; the task sits directly under the pane's title.
    Plain,
    /// One list's tasks in a smart view.
    List(String),
    Overdue,
    Today,
    Tomorrow,
    ThisWeek,
    Later,
    Completed,
}

impl Group {
    /// The key fold state is kept under.
    pub fn key(&self) -> String {
        match self {
            Group::Plain => "plain".to_owned(),
            Group::List(name) => format!("list:{name}"),
            Group::Overdue => "planned:overdue".to_owned(),
            Group::Today => "planned:today".to_owned(),
            Group::Tomorrow => "planned:tomorrow".to_owned(),
            Group::ThisWeek => "planned:this-week".to_owned(),
            Group::Later => "planned:later".to_owned(),
            Group::Completed => "completed".to_owned(),
        }
    }

    /// The heading shown, if the group has one.
    pub fn title(&self) -> Option<String> {
        match self {
            Group::Plain => None,
            Group::List(name) => Some(name.clone()),
            Group::Overdue => Some("Overdue".to_owned()),
            Group::Today => Some("Today".to_owned()),
            Group::Tomorrow => Some("Tomorrow".to_owned()),
            Group::ThisWeek => Some("This Week".to_owned()),
            Group::Later => Some("Later".to_owned()),
            Group::Completed => Some("Completed".to_owned()),
        }
    }
}

/// The group `task` belongs to in `view`, with `today` deciding the planned
/// buckets.
pub fn group_of(task: &Task, view: &View, list_name: &str, today: Date) -> Group {
    if !task.status.is_open() {
        return Group::Completed;
    }
    match view {
        View::Smart(smart) if smart.planned() => match task.due {
            None => Group::Later,
            Some(due) => planned_group(due, today),
        },
        View::Smart(_) => Group::List(list_name.to_owned()),
        View::List { .. } => Group::Plain,
    }
}

/// Where a due date falls among the planned groups.
pub fn planned_group(due: Date, today: Date) -> Group {
    if due < today {
        Group::Overdue
    } else if due == today {
        Group::Today
    } else if due == today.tomorrow().unwrap_or(today) {
        Group::Tomorrow
    } else if due <= today.checked_add(Span::new().days(6)).unwrap_or(today) {
        Group::ThisWeek
    } else {
        Group::Later
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Importance, ListId, Status, Task, TaskId};
    use crate::tui::app::{Smart, View};
    use jiff::{Timestamp, civil::date};

    fn task(id: &str, due: Option<Date>, status: Status) -> Task {
        let at = Timestamp::from_second(1_700_000_000).unwrap();
        Task {
            id: TaskId(id.to_owned()),
            list: ListId("l1".to_owned()),
            title: id.to_owned(),
            status,
            importance: Importance::Normal,
            due,
            reminder: None,
            completed_at: None,
            created_at: at,
            modified_at: at,
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
    fn completed_tasks_go_under_completed_whatever_the_view() {
        let view = View::Smart(Smart::Planned);
        let task = task("t1", Some(date(2026, 11, 20)), Status::Completed);

        assert_eq!(
            group_of(&task, &view, "Groceries", date(2026, 11, 20)),
            Group::Completed
        );
    }

    #[test]
    fn planned_tasks_are_grouped_by_how_soon_they_are_due() {
        let today = date(2026, 11, 20);
        let view = View::Smart(Smart::Planned);

        let of = |due| {
            group_of(
                &task("t", Some(due), Status::NotStarted),
                &view,
                "Work",
                today,
            )
        };

        assert_eq!(of(date(2026, 11, 19)), Group::Overdue);
        assert_eq!(of(today), Group::Today);
        assert_eq!(of(date(2026, 11, 21)), Group::Tomorrow);
        assert_eq!(of(date(2026, 11, 25)), Group::ThisWeek);
        assert_eq!(of(date(2026, 12, 1)), Group::Later);
    }

    #[test]
    fn important_and_all_group_by_list_and_a_real_list_has_no_heading() {
        let task = task("t1", None, Status::NotStarted);
        let today = date(2026, 11, 20);

        assert_eq!(
            group_of(&task, &View::Smart(Smart::Important), "Work", today),
            Group::List("Work".to_owned())
        );
        assert_eq!(
            group_of(
                &task,
                &View::List {
                    account: 0,
                    id: ListId("l1".to_owned())
                },
                "Work",
                today
            ),
            Group::Plain
        );
    }
}
