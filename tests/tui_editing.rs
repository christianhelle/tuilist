mod support;

use std::time::Duration;

use tokio::sync::mpsc;
use tuilist::{
    core::{Command, fake::FakeBackend, run_engine, sync_lists, sync_tasks},
    model::{BackendError, ListId, Status, TaskDraft, TaskId},
    store::Store,
    tui::{
        app::{AppState, Effect, Status as AppStatus},
        taskboxes::TaskBoxes,
    },
};

use support::{list, task};

#[tokio::test]
async fn a_failed_write_reverts_the_row_and_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.db");
    let reader = Store::open(&path).unwrap();
    let mut seeded = Store::open(&path).unwrap();
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[task("t1", "l1")]);
    sync_lists(&backend, &mut seeded).await.unwrap();
    sync_tasks(&backend, &mut seeded, &ListId("l1".to_owned()))
        .await
        .unwrap();

    let (commands, command_rx) = mpsc::unbounded_channel();
    let (events, event_rx) = mpsc::channel(64);
    let engine = tokio::spawn(run_engine(backend.clone(), seeded, command_rx, events));
    let mut boxes = TaskBoxes::new(vec![("ada@example.com".to_owned(), reader)]);
    boxes.attach(0, commands, event_rx);
    let mut app = AppState::default();
    boxes.load(&mut app).unwrap();
    assert_eq!(app.tasks[0].task.status, Status::NotStarted);

    backend.fail_next(
        "update_task",
        BackendError::Other("no connection".to_owned()),
    );
    boxes.apply(
        &mut app,
        Effect::Send {
            account: 0,
            command: Command::UpdateTasks {
                tasks: vec![(ListId("l1".to_owned()), TaskId("t1".to_owned()))],
                draft: TaskDraft {
                    completed: Some(true),
                    ..TaskDraft::default()
                },
            },
        },
    );

    for _ in 0..100 {
        boxes.poll(&mut app);
        if app.status.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let Some(AppStatus::Error(reason)) = &app.status else {
        panic!("expected an error, got {:?}", app.status);
    };
    assert!(reason.contains("saving a task"), "{reason}");
    assert!(reason.contains("no connection"), "{reason}");
    assert_eq!(app.tasks[0].task.status, Status::NotStarted);
    engine.abort();
}
