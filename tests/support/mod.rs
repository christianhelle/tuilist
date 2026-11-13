#![allow(dead_code)]

use jiff::Timestamp;
use tuilist::model::{Importance, ListId, Status, Step, StepId, Task, TaskId, TaskList};

pub fn list(id: &str, name: &str) -> TaskList {
    TaskList {
        id: ListId(id.to_owned()),
        name: name.to_owned(),
        well_known: None,
        is_owner: true,
        is_shared: false,
        open: 0,
    }
}

pub fn task(id: &str, list: &str) -> Task {
    let at = Timestamp::from_second(1_700_000_000).unwrap();
    Task {
        id: TaskId(id.to_owned()),
        list: ListId(list.to_owned()),
        title: id.to_owned(),
        status: Status::NotStarted,
        importance: Importance::Normal,
        due: None,
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

pub fn step(id: &str, task: &str, checked: bool) -> Step {
    Step {
        id: StepId(id.to_owned()),
        task: TaskId(task.to_owned()),
        name: id.to_owned(),
        checked,
        created_at: Timestamp::from_second(1_700_000_000).unwrap(),
    }
}
