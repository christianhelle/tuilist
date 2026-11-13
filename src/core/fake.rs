//! A task service held in memory, for `--demo` and for tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use jiff::Timestamp;

use crate::model::{
    BackendError, DeltaPage, Importance, ListId, Status, Step, StepId, Task, TaskDraft, TaskId,
    TaskList, TodoBackend, WellKnown,
};

#[derive(Default)]
struct State {
    lists: Vec<TaskList>,
    tasks: Vec<Task>,
    steps: Vec<Step>,
    next_id: u64,
    delta_expired: bool,
}

struct Failure {
    operation: &'static str,
    error: BackendError,
}

/// A backend that keeps everything in memory and can be told to fail once.
#[derive(Clone, Default)]
pub struct FakeBackend {
    state: Arc<Mutex<State>>,
    failures: Arc<Mutex<VecDeque<Failure>>>,
}

impl FakeBackend {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds a list, as if Graph had returned it.
    pub fn with_lists(self, lists: &[TaskList]) -> Self {
        self.state.lock().unwrap().lists.extend_from_slice(lists);
        self
    }

    /// Seeds a task, as if Graph had returned it.
    pub fn with_tasks(self, tasks: &[Task]) -> Self {
        self.state.lock().unwrap().tasks.extend_from_slice(tasks);
        self
    }

    /// Seeds steps for a task, as if they had been fetched.
    pub fn with_steps(self, steps: &[Step]) -> Self {
        self.state.lock().unwrap().steps.extend_from_slice(steps);
        self
    }

    /// Makes the next call to `operation` fail with `error`.
    pub fn fail_next(&self, operation: &'static str, error: BackendError) {
        self.failures
            .lock()
            .unwrap()
            .push_back(Failure { operation, error });
    }

    /// Makes every stored delta link too old, as Graph does after a long
    /// time away.
    pub fn expire_delta_links(&self) {
        self.state.lock().unwrap().delta_expired = true;
    }

    /// The lists the fake holds.
    pub fn lists(&self) -> Vec<TaskList> {
        self.state.lock().unwrap().lists.clone()
    }

    /// The tasks the fake holds.
    pub fn tasks(&self) -> Vec<Task> {
        self.state.lock().unwrap().tasks.clone()
    }

    /// The steps the fake holds for `task`.
    pub fn steps_of(&self, task: &TaskId) -> Vec<Step> {
        self.state
            .lock()
            .unwrap()
            .steps
            .iter()
            .filter(|step| &step.task == task)
            .cloned()
            .collect()
    }

    fn fail(&self, operation: &'static str) -> Result<(), BackendError> {
        let mut failures = self.failures.lock().unwrap();
        if let Some(index) = failures
            .iter()
            .position(|failure| failure.operation == operation)
        {
            return Err(failures.remove(index).unwrap().error);
        }
        Ok(())
    }

    fn next_id(&self, prefix: &str) -> String {
        let mut state = self.state.lock().unwrap();
        state.next_id += 1;
        format!("{prefix}-{}", state.next_id)
    }
}

impl TodoBackend for FakeBackend {
    async fn lists_delta(
        &self,
        link: Option<&str>,
    ) -> Result<DeltaPage<TaskList, ListId>, BackendError> {
        self.fail("lists_delta")?;
        let state = self.state.lock().unwrap();
        if link.is_some() && state.delta_expired {
            return Err(BackendError::DeltaExpired);
        }
        let lists = if link.is_some() {
            Vec::new()
        } else {
            state.lists.clone()
        };
        drop(state);
        Ok(DeltaPage {
            upserted: lists,
            removed: Vec::new(),
            delta_link: Some("fake-lists-delta".to_owned()),
        })
    }

    async fn tasks_delta(
        &self,
        list: &ListId,
        link: Option<&str>,
    ) -> Result<DeltaPage<Task, TaskId>, BackendError> {
        self.fail("tasks_delta")?;
        let state = self.state.lock().unwrap();
        if link.is_some() && state.delta_expired {
            return Err(BackendError::DeltaExpired);
        }
        let tasks = if link.is_some() {
            Vec::new()
        } else {
            state
                .tasks
                .iter()
                .filter(|task| &task.list == list)
                .cloned()
                .collect()
        };
        Ok(DeltaPage {
            upserted: tasks,
            removed: Vec::new(),
            delta_link: Some(format!("fake-tasks-delta:{}", list.0)),
        })
    }

    async fn create_list(&self, name: &str) -> Result<TaskList, BackendError> {
        self.fail("create_list")?;
        let id = self.next_id("list");
        let list = TaskList {
            id: ListId(id),
            name: name.to_owned(),
            well_known: None,
            is_owner: true,
            is_shared: false,
            open: 0,
        };
        self.state.lock().unwrap().lists.push(list.clone());
        Ok(list)
    }

    async fn rename_list(&self, list: &ListId, name: &str) -> Result<(), BackendError> {
        self.fail("rename_list")?;
        let mut state = self.state.lock().unwrap();
        let found = state
            .lists
            .iter_mut()
            .find(|candidate| &candidate.id == list)
            .ok_or(BackendError::NotFound)?;
        found.name = name.to_owned();
        Ok(())
    }

    async fn delete_list(&self, list: &ListId) -> Result<(), BackendError> {
        self.fail("delete_list")?;
        let mut state = self.state.lock().unwrap();
        state.lists.retain(|candidate| &candidate.id != list);
        state.tasks.retain(|task| &task.list != list);
        Ok(())
    }

    async fn create_task(&self, list: &ListId, draft: &TaskDraft) -> Result<Task, BackendError> {
        self.fail("create_task")?;
        let id = self.next_id("task");
        let now = Timestamp::now();
        let mut task = Task {
            id: TaskId(id),
            list: list.clone(),
            title: String::new(),
            status: Status::NotStarted,
            importance: Importance::Normal,
            due: None,
            reminder: None,
            completed_at: None,
            created_at: now,
            modified_at: now,
            notes: String::new(),
            notes_html: false,
            recurring: false,
            has_attachments: false,
            steps_done: 0,
            steps_total: 0,
            steps_loaded: false,
        };
        apply_draft(&mut task, draft);
        self.state.lock().unwrap().tasks.push(task.clone());
        Ok(task)
    }

    async fn update_task(
        &self,
        _list: &ListId,
        task: &TaskId,
        draft: &TaskDraft,
    ) -> Result<Task, BackendError> {
        self.fail("update_task")?;
        let mut state = self.state.lock().unwrap();
        let found = state
            .tasks
            .iter_mut()
            .find(|candidate| &candidate.id == task)
            .ok_or(BackendError::NotFound)?;
        apply_draft(found, draft);
        found.modified_at = Timestamp::now();
        Ok(found.clone())
    }

    async fn delete_task(&self, _list: &ListId, task: &TaskId) -> Result<(), BackendError> {
        self.fail("delete_task")?;
        let mut state = self.state.lock().unwrap();
        state.tasks.retain(|candidate| &candidate.id != task);
        state.steps.retain(|step| &step.task != task);
        Ok(())
    }

    async fn steps(&self, _list: &ListId, task: &TaskId) -> Result<Vec<Step>, BackendError> {
        self.fail("steps")?;
        Ok(self.steps_of(task))
    }

    async fn create_step(
        &self,
        _list: &ListId,
        task: &TaskId,
        name: &str,
    ) -> Result<Step, BackendError> {
        self.fail("create_step")?;
        let id = self.next_id("step");
        let step = Step {
            id: StepId(id),
            task: task.clone(),
            name: name.to_owned(),
            checked: false,
            created_at: Timestamp::now(),
        };
        self.state.lock().unwrap().steps.push(step.clone());
        Ok(step)
    }

    async fn update_step(
        &self,
        _list: &ListId,
        _task: &TaskId,
        step: &StepId,
        name: Option<&str>,
        checked: Option<bool>,
    ) -> Result<(), BackendError> {
        self.fail("update_step")?;
        let mut state = self.state.lock().unwrap();
        let found = state
            .steps
            .iter_mut()
            .find(|candidate| &candidate.id == step)
            .ok_or(BackendError::NotFound)?;
        if let Some(name) = name {
            found.name = name.to_owned();
        }
        if let Some(checked) = checked {
            found.checked = checked;
        }
        Ok(())
    }

    async fn delete_step(
        &self,
        _list: &ListId,
        _task: &TaskId,
        step: &StepId,
    ) -> Result<(), BackendError> {
        self.fail("delete_step")?;
        let mut state = self.state.lock().unwrap();
        state.steps.retain(|candidate| &candidate.id != step);
        Ok(())
    }
}

fn apply_draft(task: &mut Task, draft: &TaskDraft) {
    if let Some(title) = &draft.title {
        task.title = title.clone();
    }
    if let Some(starred) = draft.starred {
        task.importance = if starred {
            Importance::High
        } else {
            Importance::Normal
        };
    }
    if let Some(completed) = draft.completed {
        task.status = if completed {
            Status::Completed
        } else {
            Status::NotStarted
        };
        task.completed_at = completed.then(Timestamp::now);
    }
    if let Some(due) = draft.due {
        task.due = due;
    }
    if let Some(reminder) = draft.reminder {
        task.reminder = reminder;
    }
    if let Some(notes) = &draft.notes {
        task.notes = notes.clone();
        task.notes_html = false;
    }
}

/// A list to seed a fake with.
pub fn list(id: &str) -> TaskList {
    TaskList {
        id: ListId(id.to_owned()),
        name: id.to_owned(),
        well_known: None,
        is_owner: true,
        is_shared: false,
        open: 0,
    }
}

/// A task to seed a fake with, in a list called `tasks`.
pub fn task(id: &str) -> Task {
    task_in(id, "tasks")
}

/// A task to seed a fake with, in the list `list`.
pub fn task_in(id: &str, list: &str) -> Task {
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

/// A well-known list, for demos that need Microsoft's own lists.
pub fn well_known(id: &str, name: &str, kind: WellKnown) -> TaskList {
    TaskList {
        name: name.to_owned(),
        well_known: Some(kind),
        ..list(id)
    }
}
