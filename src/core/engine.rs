//! The engine loop: applies writes optimistically, talks to the backend and
//! tells the UI what changed.

use tokio::sync::mpsc::{Sender, UnboundedReceiver};

use crate::{
    core::sync::{sync_lists, sync_tasks},
    model::{Event, ListId, Step, StepId, Task, TaskDraft, TaskId, TodoBackend, sort_lists},
    store::Store,
};

#[derive(Debug, Clone)]
pub enum Command {
    /// Lists, then every list's tasks.
    SyncAll,
    SyncList(ListId),
    LoadSteps {
        list: ListId,
        task: TaskId,
    },
    CreateList(String),
    RenameList {
        list: ListId,
        name: String,
    },
    DeleteList(ListId),
    CreateTask {
        list: ListId,
        draft: TaskDraft,
    },
    UpdateTasks {
        tasks: Vec<(ListId, TaskId)>,
        draft: TaskDraft,
    },
    DeleteTasks(Vec<(ListId, TaskId)>),
    MoveTasks {
        tasks: Vec<(ListId, TaskId)>,
        to: ListId,
    },
    CreateStep {
        list: ListId,
        task: TaskId,
        name: String,
    },
    UpdateStep {
        list: ListId,
        task: TaskId,
        step: StepId,
        name: Option<String>,
        checked: Option<bool>,
    },
    DeleteStep {
        list: ListId,
        task: TaskId,
        step: StepId,
    },
}

impl Command {
    /// What to say the command was doing when it fails.
    fn what(&self) -> &'static str {
        match self {
            Command::SyncAll | Command::SyncList(_) => "syncing",
            Command::LoadSteps { .. } => "loading steps",
            Command::CreateList(_) | Command::DeleteList(_) | Command::RenameList { .. } => {
                "saving a list"
            }
            Command::CreateTask { .. } | Command::UpdateTasks { .. } => "saving a task",
            Command::DeleteTasks(_) => "deleting tasks",
            Command::MoveTasks { .. } => "moving tasks",
            Command::CreateStep { .. }
            | Command::UpdateStep { .. }
            | Command::DeleteStep { .. } => "saving a step",
        }
    }
}

/// Runs until the UI drops its side of the commands channel.
pub async fn run_engine<B: TodoBackend + Send + 'static>(
    backend: B,
    store: Store,
    mut commands: UnboundedReceiver<Command>,
    events: Sender<Event>,
) {
    let mut engine = Engine {
        backend,
        store,
        events,
    };
    while let Some(command) = commands.recv().await {
        let what = command.what();
        if let Err(reason) = engine.apply(command).await {
            send(
                &engine.events,
                Event::Failed {
                    what: what.to_owned(),
                    reason,
                },
            )
            .await;
        }
    }
}

struct Engine<B> {
    backend: B,
    store: Store,
    events: Sender<Event>,
}

impl<B: TodoBackend> Engine<B> {
    async fn apply(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::SyncAll => self.sync_all().await,
            Command::SyncList(list) => self.sync_list(list).await,
            Command::LoadSteps { list, task } => self.load_steps(&list, &task).await,
            Command::CreateList(name) => self.create_list(name).await,
            Command::RenameList { list, name } => self.rename_list(list, name).await,
            Command::DeleteList(list) => self.delete_list(list).await,
            Command::CreateTask { list, draft } => self.create_task(list, draft).await,
            Command::UpdateTasks { tasks, draft } => self.update_tasks(tasks, draft).await,
            Command::DeleteTasks(tasks) => self.delete_tasks(tasks).await,
            Command::MoveTasks { tasks, to } => self.move_tasks(tasks, to).await,
            Command::CreateStep { list, task, name } => self.create_step(list, task, name).await,
            Command::UpdateStep {
                list,
                task,
                step,
                name,
                checked,
            } => self.update_step(list, task, step, name, checked).await,
            Command::DeleteStep { list, task, step } => self.delete_step(list, task, step).await,
        }
    }

    async fn sync_all(&mut self) -> Result<(), String> {
        if let Err(error) = sync_lists(&self.backend, &mut self.store).await {
            failed(&self.events, "syncing lists", describe(&error)).await;
        } else {
            send(&self.events, Event::ListsSynced).await;
        }
        let mut lists = self.store.lists().map_err(store_error)?;
        sort_lists(&mut lists);
        let mut changed = Vec::new();
        for list in &lists {
            match sync_tasks(&self.backend, &mut self.store, &list.id).await {
                Ok(()) => {
                    changed.push(list.id.clone());
                    send(&self.events, Event::ListSynced(list.id.clone())).await;
                }
                Err(error) => {
                    failed(
                        &self.events,
                        &format!("syncing {}", list.name),
                        describe(&error),
                    )
                    .await;
                }
            }
        }
        if !changed.is_empty() {
            send(&self.events, Event::TasksChanged(changed)).await;
        }
        send(&self.events, Event::SyncFinished).await;
        Ok(())
    }

    async fn sync_list(&mut self, list: ListId) -> Result<(), String> {
        sync_tasks(&self.backend, &mut self.store, &list)
            .await
            .map_err(describe)?;
        send(&self.events, Event::ListSynced(list.clone())).await;
        send(&self.events, Event::TasksChanged(vec![list])).await;
        Ok(())
    }

    async fn load_steps(&mut self, list: &ListId, task: &TaskId) -> Result<(), String> {
        let steps = self.backend.steps(list, task).await.map_err(describe)?;
        self.store.put_steps(task, &steps).map_err(store_error)?;
        send(&self.events, Event::StepsLoaded(task.clone())).await;
        Ok(())
    }

    async fn create_list(&mut self, name: String) -> Result<(), String> {
        let list = self.backend.create_list(&name).await.map_err(describe)?;
        self.store
            .upsert_lists(std::slice::from_ref(&list))
            .map_err(store_error)?;
        send(&self.events, Event::ListsSynced).await;
        Ok(())
    }

    async fn rename_list(&mut self, list: ListId, name: String) -> Result<(), String> {
        let before = self.store.lists().map_err(store_error)?;
        let Some(mut renamed) = before.iter().find(|l| l.id == list).cloned() else {
            return Err("that list is no longer there".to_owned());
        };
        renamed.name = name.clone();
        self.store
            .upsert_lists(std::slice::from_ref(&renamed))
            .map_err(store_error)?;
        send(&self.events, Event::ListsSynced).await;
        if let Err(error) = self.backend.rename_list(&list, &name).await {
            self.store.upsert_lists(&before).map_err(store_error)?;
            send(&self.events, Event::ListsSynced).await;
            return Err(describe(&error));
        }
        Ok(())
    }

    async fn delete_list(&mut self, list: ListId) -> Result<(), String> {
        let lists = self.store.lists().map_err(store_error)?;
        let Some(gone) = lists.iter().find(|l| l.id == list).cloned() else {
            return Ok(());
        };
        let tasks = self.store.tasks_in(&list).map_err(store_error)?;
        let steps: Vec<(TaskId, Vec<Step>)> = tasks
            .iter()
            .filter(|task| task.steps_loaded)
            .map(|task| {
                self.store
                    .steps(&task.id)
                    .map(|steps| (task.id.clone(), steps))
                    .map_err(store_error)
            })
            .collect::<Result<_, String>>()?;
        self.store
            .remove_lists(std::slice::from_ref(&list))
            .map_err(store_error)?;
        send(&self.events, Event::ListsSynced).await;
        send(&self.events, Event::TasksChanged(vec![list.clone()])).await;
        if let Err(error) = self.backend.delete_list(&list).await {
            self.store
                .upsert_lists(std::slice::from_ref(&gone))
                .map_err(store_error)?;
            self.store.upsert_tasks(&tasks).map_err(store_error)?;
            for (task, steps) in &steps {
                self.store.put_steps(task, steps).map_err(store_error)?;
            }
            send(&self.events, Event::ListsSynced).await;
            send(&self.events, Event::TasksChanged(vec![list])).await;
            return Err(describe(&error));
        }
        Ok(())
    }

    async fn create_task(&mut self, list: ListId, draft: TaskDraft) -> Result<(), String> {
        let task = self
            .backend
            .create_task(&list, &draft)
            .await
            .map_err(describe)?;
        self.store
            .upsert_tasks(std::slice::from_ref(&task))
            .map_err(store_error)?;
        send(&self.events, Event::TasksChanged(vec![list])).await;
        Ok(())
    }

    async fn update_tasks(
        &mut self,
        tasks: Vec<(ListId, TaskId)>,
        draft: TaskDraft,
    ) -> Result<(), String> {
        let mut before = Vec::new();
        let mut lists = Vec::new();
        for (list, id) in &tasks {
            if let Some(previous) = self.store.task(id).map_err(store_error)? {
                self.store.change_task(id, &draft).map_err(store_error)?;
                before.push(previous);
            }
            if !lists.contains(list) {
                lists.push(list.clone());
            }
        }
        send(&self.events, Event::TasksChanged(lists.clone())).await;

        for (list, id) in &tasks {
            match self.backend.update_task(list, id, &draft).await {
                Ok(saved) => {
                    self.store
                        .upsert_tasks(std::slice::from_ref(&saved))
                        .map_err(store_error)?;
                    if saved.recurring && draft.completed.is_some() {
                        self.resync_recurring(list).await;
                    }
                }
                Err(error) => {
                    if let Some(previous) = before.iter().find(|task| &task.id == id) {
                        self.store
                            .upsert_tasks(std::slice::from_ref(previous))
                            .map_err(store_error)?;
                    }
                    send(&self.events, Event::TasksChanged(vec![list.clone()])).await;
                    failed(&self.events, "saving a task", describe(&error)).await;
                }
            }
        }
        Ok(())
    }

    /// Completing a recurring task makes the server create the next
    /// occurrence, so the list is fetched again straight away.
    async fn resync_recurring(&mut self, list: &ListId) {
        match sync_tasks(&self.backend, &mut self.store, list).await {
            Ok(()) => {
                send(&self.events, Event::ListSynced(list.clone())).await;
                send(&self.events, Event::TasksChanged(vec![list.clone()])).await;
            }
            Err(error) => failed(&self.events, "syncing a recurring task", describe(&error)).await,
        }
    }

    async fn delete_tasks(&mut self, tasks: Vec<(ListId, TaskId)>) -> Result<(), String> {
        let mut before = Vec::new();
        let mut lists = Vec::new();
        for (list, id) in &tasks {
            if let Some(task) = self.store.task(id).map_err(store_error)? {
                let steps = if task.steps_loaded {
                    self.store.steps(id).map_err(store_error)?
                } else {
                    Vec::new()
                };
                before.push((task, steps));
                self.store
                    .remove_tasks(std::slice::from_ref(id))
                    .map_err(store_error)?;
            }
            if !lists.contains(list) {
                lists.push(list.clone());
            }
        }
        send(&self.events, Event::TasksChanged(lists.clone())).await;

        for (list, id) in &tasks {
            if let Err(error) = self.backend.delete_task(list, id).await {
                if let Some((task, steps)) = before.iter().find(|(task, _)| &task.id == id) {
                    self.store
                        .upsert_tasks(std::slice::from_ref(task))
                        .map_err(store_error)?;
                    if task.steps_loaded {
                        self.store.put_steps(id, steps).map_err(store_error)?;
                    }
                }
                send(&self.events, Event::TasksChanged(vec![list.clone()])).await;
                failed(&self.events, "deleting tasks", describe(&error)).await;
            }
        }
        Ok(())
    }

    async fn move_tasks(&mut self, tasks: Vec<(ListId, TaskId)>, to: ListId) -> Result<(), String> {
        let mut copies = Vec::new();
        for (list, id) in &tasks {
            let Some(task) = self.store.task(id).map_err(store_error)? else {
                continue;
            };
            let steps = if task.steps_loaded {
                self.store.steps(id).map_err(store_error)?
            } else {
                self.backend.steps(list, id).await.map_err(describe)?
            };
            match self.copy_task(&task, &to, &steps).await {
                Ok((copy, copied_steps)) => copies.push((copy, copied_steps)),
                Err(error) => {
                    // Leave the originals alone and take back anything made.
                    for (copy, _) in copies {
                        let _ = self.backend.delete_task(&copy.list, &copy.id).await;
                    }
                    return Err(error);
                }
            }
        }

        let mut lists: Vec<ListId> = Vec::new();
        for (list, id) in &tasks {
            match self.backend.delete_task(list, id).await {
                Ok(()) => {
                    self.store
                        .remove_tasks(std::slice::from_ref(id))
                        .map_err(store_error)?;
                }
                Err(error) => failed(&self.events, "moving tasks", describe(&error)).await,
            }
            if !lists.contains(list) {
                lists.push(list.clone());
            }
        }
        for (copy, steps) in &copies {
            self.store
                .upsert_tasks(std::slice::from_ref(copy))
                .map_err(store_error)?;
            if !steps.is_empty() {
                self.store.put_steps(&copy.id, steps).map_err(store_error)?;
            }
        }
        if !lists.contains(&to) {
            lists.push(to);
        }
        send(&self.events, Event::TasksChanged(lists)).await;
        Ok(())
    }

    /// Makes a task in `to` with the same fields and steps as `task`, and
    /// returns it with the steps made.
    async fn copy_task(
        &mut self,
        task: &Task,
        to: &ListId,
        steps: &[Step],
    ) -> Result<(Task, Vec<Step>), String> {
        let draft = TaskDraft {
            title: Some(task.title.clone()),
            starred: Some(task.importance.is_starred()),
            completed: Some(!task.status.is_open()),
            due: Some(task.due),
            reminder: Some(task.reminder),
            notes: Some(task.notes.clone()),
        };
        let copy = self
            .backend
            .create_task(to, &draft)
            .await
            .map_err(describe)?;
        let mut copied_steps = Vec::new();
        for step in steps {
            match self.backend.create_step(to, &copy.id, &step.name).await {
                Ok(mut copied) => {
                    if step.checked {
                        let _ = self
                            .backend
                            .update_step(to, &copy.id, &copied.id, None, Some(true))
                            .await;
                        copied.checked = true;
                    }
                    copied_steps.push(copied);
                }
                Err(error) => return Err(describe(&error)),
            }
        }
        Ok((copy, copied_steps))
    }

    async fn create_step(
        &mut self,
        list: ListId,
        task: TaskId,
        name: String,
    ) -> Result<(), String> {
        let step = self
            .backend
            .create_step(&list, &task, &name)
            .await
            .map_err(describe)?;
        self.store.upsert_step(&step).map_err(store_error)?;
        send(&self.events, Event::StepsLoaded(task)).await;
        send(&self.events, Event::TasksChanged(vec![list])).await;
        Ok(())
    }

    async fn update_step(
        &mut self,
        list: ListId,
        task: TaskId,
        step: StepId,
        name: Option<String>,
        checked: Option<bool>,
    ) -> Result<(), String> {
        let before = self
            .store
            .steps(&task)
            .map_err(store_error)?
            .into_iter()
            .find(|candidate| candidate.id == step);
        if let Some(previous) = &before {
            let mut changed = previous.clone();
            if let Some(name) = &name {
                changed.name = name.clone();
            }
            if let Some(checked) = checked {
                changed.checked = checked;
            }
            self.store.upsert_step(&changed).map_err(store_error)?;
        }
        send(&self.events, Event::StepsLoaded(task.clone())).await;
        send(&self.events, Event::TasksChanged(vec![list.clone()])).await;
        if let Err(error) = self
            .backend
            .update_step(&list, &task, &step, name.as_deref(), checked)
            .await
        {
            if let Some(previous) = &before {
                self.store.upsert_step(previous).map_err(store_error)?;
            }
            send(&self.events, Event::StepsLoaded(task)).await;
            send(&self.events, Event::TasksChanged(vec![list])).await;
            return Err(describe(&error));
        }
        Ok(())
    }

    async fn delete_step(
        &mut self,
        list: ListId,
        task: TaskId,
        step: StepId,
    ) -> Result<(), String> {
        let before = self
            .store
            .steps(&task)
            .map_err(store_error)?
            .into_iter()
            .find(|candidate| candidate.id == step);
        self.store.remove_step(&step).map_err(store_error)?;
        send(&self.events, Event::StepsLoaded(task.clone())).await;
        send(&self.events, Event::TasksChanged(vec![list.clone()])).await;
        if let Err(error) = self.backend.delete_step(&list, &task, &step).await {
            if let Some(previous) = before {
                self.store.upsert_step(&previous).map_err(store_error)?;
            }
            send(&self.events, Event::StepsLoaded(task)).await;
            send(&self.events, Event::TasksChanged(vec![list])).await;
            return Err(describe(&error));
        }
        Ok(())
    }
}

fn describe(error: impl ToString) -> String {
    error.to_string()
}

async fn send(events: &Sender<Event>, event: Event) {
    let _ = events.send(event).await;
}

async fn failed(events: &Sender<Event>, what: &str, reason: impl ToString) {
    send(
        events,
        Event::Failed {
            what: what.to_owned(),
            reason: reason.to_string(),
        },
    )
    .await;
}

fn store_error(error: crate::store::StoreError) -> String {
    error.to_string()
}
