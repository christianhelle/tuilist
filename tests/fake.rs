mod support;

use jiff::civil::date;
use tuilist::core::fake::{self, FakeBackend};
use tuilist::model::{BackendError, ListId, TaskDraft, TaskId, TodoBackend};

use support::list;

#[test]
fn a_fake_returns_its_lists_once_then_only_on_request() {
    let backend = FakeBackend::new().with_lists(&[list("l1", "Groceries")]);

    let first = futures::executor::block_on(backend.lists_delta(None)).unwrap();
    let later = futures::executor::block_on(backend.lists_delta(Some("link"))).unwrap();

    assert_eq!(first.upserted.len(), 1);
    assert_eq!(first.delta_link.as_deref(), Some("fake-lists-delta"));
    assert!(later.upserted.is_empty());
}

#[test]
fn an_expired_delta_link_is_refused_until_it_is_left_out() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1")]);
    backend.expire_delta_links();
    let list = ListId("l1".to_owned());

    let expired = futures::executor::block_on(backend.tasks_delta(&list, Some("old")));
    let fresh = futures::executor::block_on(backend.tasks_delta(&list, None)).unwrap();

    assert_eq!(expired, Err(BackendError::DeltaExpired));
    assert_eq!(fresh.upserted.len(), 1);
}

#[test]
fn a_fake_makes_tasks_from_drafts() {
    let backend = FakeBackend::new().with_lists(&[list("l1", "Groceries")]);
    let list = ListId("l1".to_owned());

    let task = futures::executor::block_on(backend.create_task(
        &list,
        &TaskDraft {
            title: Some("Milk".to_owned()),
            starred: Some(true),
            due: Some(Some(date(2026, 11, 20))),
            ..TaskDraft::default()
        },
    ))
    .unwrap();

    assert_eq!(task.title, "Milk");
    assert!(task.importance.is_starred());
    assert_eq!(task.due, Some(date(2026, 11, 20)));
    assert_eq!(backend.tasks().len(), 1);
}

#[test]
fn a_fail_next_rule_fires_once_and_only_for_its_operation() {
    let backend = FakeBackend::new().with_lists(&[list("l1", "Groceries")]);
    backend.fail_next("create_list", BackendError::Other("offline".to_owned()));
    let list = ListId("l1".to_owned());

    let renamed = futures::executor::block_on(backend.rename_list(&list, "Shopping"));
    let refused = futures::executor::block_on(backend.create_list("Work"));
    let accepted = futures::executor::block_on(backend.create_list("Home")).unwrap();

    assert!(renamed.is_ok());
    assert_eq!(refused, Err(BackendError::Other("offline".to_owned())));
    assert_eq!(accepted.name, "Home");
}

#[test]
fn deleting_a_task_takes_its_steps_with_it() {
    let backend = FakeBackend::new().with_tasks(&[fake::task_in("t1", "l1")]);
    let mut step = support::step("s1", "t1", false);
    step.task = TaskId("t1".to_owned());
    let backend = backend.with_steps(&[step]);
    let list = ListId("l1".to_owned());

    futures::executor::block_on(backend.delete_task(&list, &TaskId("t1".to_owned()))).unwrap();

    assert!(backend.tasks().is_empty());
    assert!(backend.steps_of(&TaskId("t1".to_owned())).is_empty());
}
