mod support;

use tuilist::core::fake::{self, FakeBackend};
use tuilist::core::{sync_lists, sync_tasks};
use tuilist::model::{ListId, TaskId, TodoBackend};
use tuilist::store::Store;

use support::list;

#[test]
fn syncing_brings_a_list_and_its_tasks_into_the_cache() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1")]);
    let mut store = Store::open_in_memory().unwrap();

    futures::executor::block_on(async {
        sync_lists(&backend, &mut store).await.unwrap();
        sync_tasks(&backend, &mut store, &ListId("l1".to_owned()))
            .await
            .unwrap();
    });

    assert_eq!(store.lists().unwrap().len(), 1);
    assert!(store.task(&TaskId("t1".to_owned())).unwrap().is_some());
}

#[test]
fn a_second_sync_leaves_the_cached_tasks_alone() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1")]);
    let mut store = Store::open_in_memory().unwrap();
    let list = ListId("l1".to_owned());

    futures::executor::block_on(async {
        sync_lists(&backend, &mut store).await.unwrap();
        sync_tasks(&backend, &mut store, &list).await.unwrap();
        sync_tasks(&backend, &mut store, &list).await.unwrap();
    });

    assert_eq!(store.tasks_in(&list).unwrap().len(), 1);
}

#[test]
fn an_expired_tasks_delta_link_resyncs_the_list_from_scratch() {
    let backend = FakeBackend::new()
        .with_lists(&[list("l1", "Groceries")])
        .with_tasks(&[fake::task_in("t1", "l1"), fake::task_in("t2", "l1")]);
    let mut store = Store::open_in_memory().unwrap();
    let list = ListId("l1".to_owned());

    futures::executor::block_on(async {
        sync_lists(&backend, &mut store).await.unwrap();
        sync_tasks(&backend, &mut store, &list).await.unwrap();
        backend
            .delete_task(&list, &TaskId("t2".to_owned()))
            .await
            .unwrap();
        backend.expire_delta_links();
        sync_tasks(&backend, &mut store, &list).await.unwrap();
    });

    assert!(store.task(&TaskId("t1".to_owned())).unwrap().is_some());
    assert!(store.task(&TaskId("t2".to_owned())).unwrap().is_none());
}

#[test]
fn an_expired_lists_delta_link_drops_the_lists_the_service_no_longer_returns() {
    let backend = FakeBackend::new().with_lists(&[list("l1", "Groceries"), list("l2", "Work")]);
    let mut store = Store::open_in_memory().unwrap();

    futures::executor::block_on(async {
        sync_lists(&backend, &mut store).await.unwrap();
        backend.delete_list(&ListId("l2".to_owned())).await.unwrap();
        backend.expire_delta_links();
        sync_lists(&backend, &mut store).await.unwrap();
    });

    let lists = store.lists().unwrap();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].id.0, "l1");
}
