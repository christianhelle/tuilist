//! The stores the panes read from, one per account, and what an [`Effect`]
//! asks of them.

use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::mpsc::{Receiver, UnboundedSender};

use crate::{
    core::Command,
    model::{Event, ListId, Task, TaskId, TaskList, sort_lists, sort_tasks},
    store::Store,
    tui::app::{AppState, Effect, TaskRow, View},
};

pub struct TaskBox {
    pub address: String,
    pub store: Store,
    commands: Option<UnboundedSender<Command>>,
    events: Option<Receiver<Event>>,
}

pub struct TaskBoxes {
    pub boxes: Vec<TaskBox>,
    install: Option<UnboundedSender<()>>,
}

impl TaskBoxes {
    pub fn new(accounts: Vec<(String, Store)>) -> Self {
        Self {
            boxes: accounts
                .into_iter()
                .map(|(address, store)| TaskBox {
                    address,
                    store,
                    commands: None,
                    events: None,
                })
                .collect(),
            install: None,
        }
    }

    /// Connects the update installer the UI asks to replace this binary.
    pub fn attach_installer(&mut self, install: UnboundedSender<()>) {
        self.install = Some(install);
    }

    /// Connects an account to its engine, so writes go through it.
    pub fn attach(
        &mut self,
        account: usize,
        commands: UnboundedSender<Command>,
        events: Receiver<Event>,
    ) {
        if let Some(task_box) = self.boxes.get_mut(account) {
            task_box.commands = Some(commands);
            task_box.events = Some(events);
        }
    }

    /// Shows whatever the engines have reported since the last draw.
    pub fn poll(&mut self, app: &mut AppState) {
        let mut events = Vec::new();
        for (index, task_box) in self.boxes.iter_mut().enumerate() {
            if let Some(events_in) = &mut task_box.events {
                while let Ok(event) = events_in.try_recv() {
                    events.push((index, event));
                }
            }
        }
        for (account, event) in events {
            self.on_event(app, account, event);
        }
    }

    pub fn lists(&self) -> Result<Vec<(String, Vec<TaskList>)>> {
        self.boxes
            .iter()
            .map(|account| {
                let mut lists = account.store.lists()?;
                sort_lists(&mut lists);
                Ok((account.address.clone(), lists))
            })
            .collect()
    }

    pub fn load(&mut self, app: &mut AppState) -> Result<()> {
        let lists = self.lists()?;
        app.set_accounts(lists);
        app.smart_counts = self.smart_counts()?;
        self.load_tasks(app)
    }

    fn smart_counts(&self) -> Result<(u32, u32, u32)> {
        let mut counts = (0, 0, 0);
        for account in &self.boxes {
            counts.0 += u32::try_from(account.store.important()?.len()).unwrap_or(u32::MAX);
            counts.1 += u32::try_from(account.store.planned()?.len()).unwrap_or(u32::MAX);
            counts.2 += u32::try_from(account.store.all_open()?.len()).unwrap_or(u32::MAX);
        }
        Ok(counts)
    }

    fn load_tasks(&mut self, app: &mut AppState) -> Result<()> {
        let mut rows = Vec::new();
        let query = app
            .search
            .as_ref()
            .map(|search| search.query().trim().to_owned())
            .filter(|query| !query.is_empty());
        for (index, account) in self.boxes.iter().enumerate() {
            let names: HashMap<ListId, String> = account
                .store
                .lists()?
                .into_iter()
                .map(|list| (list.id, list.name))
                .collect();
            let mut tasks = match &query {
                Some(query) => account.store.search(query, 200)?,
                None => match &app.view {
                    View::Smart(smart) => match smart {
                        crate::tui::app::Smart::Important => account.store.important()?,
                        crate::tui::app::Smart::Planned => account.store.planned()?,
                        crate::tui::app::Smart::All => account.store.all_open()?,
                    },
                    View::List { account: owner, id } if *owner == index => {
                        account.store.tasks_in(id)?
                    }
                    View::List { .. } => Vec::new(),
                },
            };
            if !matches!(app.view, View::Smart(crate::tui::app::Smart::Planned)) {
                sort_tasks(&mut tasks);
            }
            push_rows(&mut rows, index, tasks, &names);
        }
        app.set_tasks(rows);
        self.show_selected_details(app)
    }

    fn show_selected_details(&mut self, app: &mut AppState) -> Result<()> {
        let Some(row) = app.selected_task().cloned() else {
            app.show_details(None);
            return Ok(());
        };
        let steps = if row.task.steps_loaded {
            Some(self.steps(&row)?)
        } else {
            None
        };
        app.show_details(steps.as_deref());
        if !row.task.steps_loaded
            && let Some(effect) = app.load_selected_steps()
        {
            self.apply(app, effect);
        }
        Ok(())
    }

    fn steps(&self, row: &TaskRow) -> Result<Vec<crate::model::Step>> {
        Ok(self
            .boxes
            .get(row.account)
            .map_or_else(Vec::new, |account| {
                account.store.steps(&row.task.id).unwrap_or_default()
            }))
    }

    fn load_steps(&mut self, app: &mut AppState, account: usize, task: &TaskId) -> Result<()> {
        let Some(account) = self.boxes.get(account) else {
            return Ok(());
        };
        let steps = account.store.steps(task)?;
        app.set_steps(task, &steps);
        if app.selected_task().is_some_and(|row| &row.task.id == task) {
            app.show_details(Some(&steps));
        }
        Ok(())
    }

    pub fn apply(&mut self, app: &mut AppState, effect: Effect) {
        let result = match effect {
            Effect::None | Effect::Quit | Effect::Copy(_) => Ok(()),
            Effect::InstallUpdate => {
                let Some(install) = &self.install else {
                    app.status_error("Run `todo update` to install it");
                    return;
                };
                install
                    .send(())
                    .map_err(|_| anyhow::anyhow!("The update installer has stopped"))
            }
            Effect::ReloadLists => self.load(app),
            Effect::ReloadTasks => self.load_tasks(app),
            Effect::Send {
                account,
                command: Command::LoadSteps { task, .. },
            } => self.load_steps(app, account, &task),
            Effect::SendAll(Command::SyncAll) => {
                let sent = self.sync_all();
                let loaded = self.load(app);
                sent.and(loaded)
            }
            Effect::Send { account, command } => self.send(account, command),
            Effect::SendAll(command) => self.broadcast(command),
        };
        if let Err(error) = result {
            app.status_error(format!("{error}"));
        }
    }

    /// Asks every account's engine to sync, for the timer and `F5`.
    pub fn sync_all(&self) -> Result<()> {
        self.broadcast(Command::SyncAll)
    }

    fn broadcast(&self, command: Command) -> Result<()> {
        for account in 0..self.boxes.len() {
            self.send(account, command.clone())?;
        }
        Ok(())
    }

    fn send(&self, account: usize, command: Command) -> Result<()> {
        let Some(task_box) = self.boxes.get(account) else {
            anyhow::bail!("No account {account}");
        };
        let Some(commands) = &task_box.commands else {
            anyhow::bail!("This demo is read-only");
        };
        commands
            .send(command)
            .map_err(|_| anyhow::anyhow!("The account's engine has stopped"))
    }

    /// Shows what an engine reported, once the accounts are live.
    pub fn on_event(&mut self, app: &mut AppState, _account: usize, event: Event) {
        match event {
            Event::Failed { what, reason } => app.status_error(format!("{what}: {reason}")),
            Event::SignedOut => {
                app.status_error("Sign-in expired. Run `todo login`, then press F5")
            }
            Event::ListsSynced | Event::ListSynced(_) | Event::SyncFinished => {
                app.status_info("Synced");
                let _ = self.load(app);
            }
            Event::TasksChanged(_) | Event::StepsLoaded(_) => {
                let _ = self.load_tasks(app);
            }
        }
    }
}

fn push_rows(
    rows: &mut Vec<TaskRow>,
    account: usize,
    tasks: Vec<Task>,
    names: &HashMap<ListId, String>,
) {
    for task in tasks {
        let list_name = names.get(&task.list).cloned().unwrap_or_default();
        rows.push(TaskRow {
            task,
            list_name,
            account,
        });
    }
}
