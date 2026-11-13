mod support;

use std::time::Duration;

use jiff::civil::date;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tuilist::core::fake::{self, FakeBackend};
use tuilist::core::{Command, run_engine};
use tuilist::model::{BackendError, Event, ListId, StepId, TaskDraft, TaskId};
use tuilist::store::Store;

use support::{list, step};

struct Harness {
    _dir: TempDir,
    reader: Store,
    backend: FakeBackend,
    commands: mpsc::UnboundedSender<Command>,
    events: mpsc::Receiver<Event>,
    engine: JoinHandle<()>,
}

impl Harness {
    async fn new(backend: FakeBackend) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");
        let reader = Store::open(&path).unwrap();
        let store = Store::open(&path).unwrap();
        let (commands, command_rx) = mpsc::unbounded_channel();
        let (event_tx, events) = mpsc::channel(64);
        let engine = tokio::spawn(run_engine(backend.clone(), store, command_rx, event_tx));
        Self {
            _dir: dir,
            reader,
            backend,
            commands,
            events,
            engine,
        }
    }

    fn send(&self, command: Command) {
        self.commands.send(command).unwrap();
    }

    async fn next(&mut self) -> Event {
        tokio::time::timeout(Duration::from_secs(2), self.events.recv())
            .await
            .expect("the engine said something")
            .expect("the engine is still running")
    }

    async fn until(&mut self, wanted: impl Fn(&Event) -> bool) -> Vec<Event> {
        let mut seen = Vec::new();
        loop {
            let event = self.next().await;
            let done = wanted(&event);
            seen.push(event);
            if done {
                return seen;
            }
        }
    }

    async fn synced(&mut self) {
        self.send(Command::SyncAll);
        self.until(|event| *event == Event::SyncFinished).await;
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.engine.abort();
    }
}

fn groceries() -> ListId {
    ListId("l1".to_owned())
}

#[tokio::test]
async fn syncing_all_lists_and_tasks_fills_the_cache() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1")]);
    let mut harness = Harness::new(backend).await;

    harness.send(Command::SyncAll);
    let events = harness.until(|event| *event == Event::SyncFinished).await;

    assert!(events.contains(&Event::ListsSynced));
    assert!(events.contains(&Event::ListSynced(groceries())));
    assert!(
        events.iter().any(
            |event| matches!(event, Event::TasksChanged(lists) if lists.contains(&groceries()))
        )
    );
    assert!(
        harness
            .reader
            .task(&TaskId("t1".to_owned()))
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn creating_a_task_stores_what_the_service_returns() {
    let backend = FakeBackend::new().with_lists(&[list("l1", "Groceries")]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;

    harness.send(Command::CreateTask {
        list: groceries(),
        draft: TaskDraft {
            title: Some("Milk".to_owned()),
            ..TaskDraft::default()
        },
    });
    harness
        .until(|event| matches!(event, Event::TasksChanged(_)))
        .await;

    let tasks = harness.reader.tasks_in(&groceries()).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Milk");
}

#[tokio::test]
async fn a_failed_rename_puts_the_old_name_back() {
    let backend = FakeBackend::new().with_lists(&[list("l1", "Groceries")]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;

    harness.backend.fail_next(
        "rename_list",
        BackendError::Other("no connection".to_owned()),
    );
    harness.send(Command::RenameList {
        list: groceries(),
        name: "Shopping".to_owned(),
    });
    harness
        .until(|event| matches!(event, Event::Failed { .. }))
        .await;

    let lists = harness.reader.lists().unwrap();
    assert_eq!(lists[0].name, "Groceries");
}

#[tokio::test]
async fn a_failed_delete_of_a_task_puts_it_back() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1")]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;
    harness.backend.fail_next(
        "delete_task",
        BackendError::Other("no connection".to_owned()),
    );

    harness.send(Command::DeleteTasks(vec![(
        groceries(),
        TaskId("t1".to_owned()),
    )]));
    harness
        .until(|event| matches!(event, Event::Failed { .. }))
        .await;

    assert!(
        harness
            .reader
            .task(&TaskId("t1".to_owned()))
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn completing_a_recurring_task_syncs_its_list_at_once() {
    let mut repeating = fake::task_in("t1", "l1");
    repeating.recurring = true;
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[repeating]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;

    harness.send(Command::UpdateTasks {
        tasks: vec![(groceries(), TaskId("t1".to_owned()))],
        draft: TaskDraft {
            completed: Some(true),
            ..TaskDraft::default()
        },
    });
    harness
        .until(|event| *event == Event::ListSynced(groceries()))
        .await;

    let task = harness
        .reader
        .task(&TaskId("t1".to_owned()))
        .unwrap()
        .unwrap();
    assert!(!task.status.is_open());
}

#[tokio::test]
async fn moving_a_task_copies_it_then_deletes_the_original() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries"), list("l2", "Work")])
        .with_tasks(&[fake::task_in("t1", "l1")])
        .with_steps(&[step("s1", "t1", true)]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;

    harness.send(Command::MoveTasks {
        tasks: vec![(groceries(), TaskId("t1".to_owned()))],
        to: ListId("l2".to_owned()),
    });
    harness
        .until(|event| matches!(event, Event::TasksChanged(_)))
        .await;

    let remaining = harness.backend.tasks();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].list.0, "l2");
    assert_eq!(remaining[0].title, "t1");
    assert_eq!(harness.backend.steps_of(&remaining[0].id).len(), 1);
    assert!(
        harness
            .reader
            .task(&TaskId("t1".to_owned()))
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn a_move_that_cannot_copy_deletes_nothing() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries"), list("l2", "Work")])
        .with_tasks(&[fake::task_in("t1", "l1")]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;
    harness.backend.fail_next(
        "create_task",
        BackendError::Other("no connection".to_owned()),
    );

    harness.send(Command::MoveTasks {
        tasks: vec![(groceries(), TaskId("t1".to_owned()))],
        to: ListId("l2".to_owned()),
    });
    harness
        .until(|event| matches!(event, Event::Failed { .. }))
        .await;

    assert_eq!(harness.backend.tasks().len(), 1);
    assert!(
        harness
            .reader
            .task(&TaskId("t1".to_owned()))
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn a_failed_step_change_puts_the_step_back() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1")])
        .with_steps(&[step("s1", "t1", false)]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;
    harness.send(Command::LoadSteps {
        list: groceries(),
        task: TaskId("t1".to_owned()),
    });
    harness
        .until(|event| matches!(event, Event::StepsLoaded(_)))
        .await;
    harness.backend.fail_next(
        "update_step",
        BackendError::Other("no connection".to_owned()),
    );

    harness.send(Command::UpdateStep {
        list: groceries(),
        task: TaskId("t1".to_owned()),
        step: StepId("s1".to_owned()),
        name: None,
        checked: Some(true),
    });
    harness
        .until(|event| matches!(event, Event::Failed { .. }))
        .await;

    let steps = harness.reader.steps(&TaskId("t1".to_owned())).unwrap();
    assert!(!steps[0].checked);
}

#[tokio::test]
async fn a_failed_list_delete_restores_its_tasks_and_steps() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1")])
        .with_steps(&[step("s1", "t1", false)]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;
    harness.send(Command::LoadSteps {
        list: groceries(),
        task: TaskId("t1".to_owned()),
    });
    harness
        .until(|event| matches!(event, Event::StepsLoaded(_)))
        .await;
    harness.backend.fail_next(
        "delete_list",
        BackendError::Other("no connection".to_owned()),
    );

    harness.send(Command::DeleteList(groceries()));
    harness
        .until(|event| matches!(event, Event::Failed { .. }))
        .await;

    assert_eq!(harness.reader.lists().unwrap().len(), 1);
    assert!(
        harness
            .reader
            .task(&TaskId("t1".to_owned()))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        harness
            .reader
            .steps(&TaskId("t1".to_owned()))
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn updating_several_tasks_applies_the_change_to_each() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1"), fake::task_in("t2", "l1")]);
    let mut harness = Harness::new(backend).await;
    harness.synced().await;

    harness.send(Command::UpdateTasks {
        tasks: vec![
            (groceries(), TaskId("t1".to_owned())),
            (groceries(), TaskId("t2".to_owned())),
        ],
        draft: TaskDraft {
            due: Some(Some(date(2026, 11, 20))),
            ..TaskDraft::default()
        },
    });
    harness
        .until(|event| matches!(event, Event::TasksChanged(_)))
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    for id in ["t1", "t2"] {
        let task = harness
            .reader
            .task(&TaskId(id.to_owned()))
            .unwrap()
            .unwrap();
        assert_eq!(task.due, Some(date(2026, 11, 20)));
    }
}
