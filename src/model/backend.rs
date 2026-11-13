//! What the engine needs from a task service, without knowing which one.

use std::future::Future;

use crate::model::{ListId, Step, StepId, Task, TaskDraft, TaskId, TaskList};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendError {
    /// The delta link is no longer valid; the list must be fully resynced.
    #[error("the list's sync state expired")]
    DeltaExpired,
    #[error("not found")]
    NotFound,
    /// The sign-in can no longer be refreshed; the user must sign in again.
    #[error("signed out")]
    SignedOut,
    #[error("{0}")]
    Other(String),
}

/// Every change a delta query returned, merged from as many pages as it took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaPage<T, Id> {
    /// Items new to the list, or sent whole.
    pub upserted: Vec<T>,
    pub removed: Vec<Id>,
    /// Pass back on the next sync to receive only newer changes.
    pub delta_link: Option<String>,
}

impl<T, Id> Default for DeltaPage<T, Id> {
    fn default() -> Self {
        Self {
            upserted: Vec::new(),
            removed: Vec::new(),
            delta_link: None,
        }
    }
}

/// A Microsoft To Do account. Graph is the only real implementation; tests
/// and `--demo` use a fake.
pub trait TodoBackend {
    fn lists_delta(
        &self,
        link: Option<&str>,
    ) -> impl Future<Output = Result<DeltaPage<TaskList, ListId>, BackendError>> + Send;

    fn tasks_delta(
        &self,
        list: &ListId,
        link: Option<&str>,
    ) -> impl Future<Output = Result<DeltaPage<Task, TaskId>, BackendError>> + Send;

    fn create_list(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<TaskList, BackendError>> + Send;

    fn rename_list(
        &self,
        list: &ListId,
        name: &str,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;

    fn delete_list(&self, list: &ListId) -> impl Future<Output = Result<(), BackendError>> + Send;

    fn create_task(
        &self,
        list: &ListId,
        draft: &TaskDraft,
    ) -> impl Future<Output = Result<Task, BackendError>> + Send;

    fn update_task(
        &self,
        list: &ListId,
        task: &TaskId,
        draft: &TaskDraft,
    ) -> impl Future<Output = Result<Task, BackendError>> + Send;

    fn delete_task(
        &self,
        list: &ListId,
        task: &TaskId,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;

    fn steps(
        &self,
        list: &ListId,
        task: &TaskId,
    ) -> impl Future<Output = Result<Vec<Step>, BackendError>> + Send;

    fn create_step(
        &self,
        list: &ListId,
        task: &TaskId,
        name: &str,
    ) -> impl Future<Output = Result<Step, BackendError>> + Send;

    fn update_step(
        &self,
        list: &ListId,
        task: &TaskId,
        step: &StepId,
        name: Option<&str>,
        checked: Option<bool>,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;

    fn delete_step(
        &self,
        list: &ListId,
        task: &TaskId,
        step: &StepId,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;
}
