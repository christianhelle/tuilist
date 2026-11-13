//! Checklist items, one of Microsoft To Do's few ways to break a task down.

use jiff::Timestamp;

use crate::model::TaskId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub id: StepId,
    pub task: TaskId,
    pub name: String,
    pub checked: bool,
    pub created_at: Timestamp,
}
