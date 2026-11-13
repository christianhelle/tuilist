mod support;

use jiff::{Timestamp, civil::date};
use tuilist::model::{Importance, ListId, Status, StepId, TaskDraft, TaskId, WellKnown};
use tuilist::store::Store;

use support::{list, step, task};

#[test]
fn lists_keep_the_number_of_open_tasks_in_them() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    let mut open = task("t1", "l1");
    open.title = "Milk".to_owned();
    let mut done = task("t2", "l1");
    done.status = Status::Completed;
    store.upsert_tasks(&[open, done]).unwrap();

    let lists = store.lists().unwrap();

    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].open, 1);
}

#[test]
fn a_list_can_be_renamed_and_its_well_known_kind_kept() {
    let mut store = Store::open_in_memory().unwrap();
    let mut default = list("l1", "Tasks");
    default.well_known = Some(WellKnown::DefaultList);
    store.upsert_lists(&[default]).unwrap();
    let mut renamed = list("l1", "My tasks");
    renamed.well_known = Some(WellKnown::DefaultList);
    store.upsert_lists(&[renamed]).unwrap();

    let lists = store.lists().unwrap();

    assert_eq!(lists[0].name, "My tasks");
    assert_eq!(lists[0].well_known, Some(WellKnown::DefaultList));
}

#[test]
fn deleting_a_list_deletes_its_tasks_and_steps() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    store.upsert_tasks(&[task("t1", "l1")]).unwrap();
    store
        .put_steps(&TaskId("t1".to_owned()), &[step("s1", "t1", false)])
        .unwrap();

    store.remove_lists(&[ListId("l1".to_owned())]).unwrap();

    assert!(store.lists().unwrap().is_empty());
    assert!(store.task(&TaskId("t1".to_owned())).unwrap().is_none());
    assert!(store.steps(&TaskId("t1".to_owned())).unwrap().is_empty());
}

#[test]
fn retaining_lists_deletes_the_ones_no_longer_returned() {
    let mut store = Store::open_in_memory().unwrap();
    store
        .upsert_lists(&[list("l1", "Groceries"), list("l2", "Work")])
        .unwrap();
    store
        .upsert_tasks(&[task("t1", "l1"), task("t2", "l2")])
        .unwrap();

    store.retain_lists(&[ListId("l1".to_owned())]).unwrap();

    let lists = store.lists().unwrap();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].id.0, "l1");
    assert_eq!(store.all_open().unwrap().len(), 1);
}

#[test]
fn changing_a_task_updates_only_what_the_draft_carries() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    let mut milk = task("t1", "l1");
    milk.title = "Milk".to_owned();
    milk.notes = "Semi-skimmed".to_owned();
    store.upsert_tasks(&[milk]).unwrap();

    store
        .change_task(
            &TaskId("t1".to_owned()),
            &TaskDraft {
                title: Some("Oat milk".to_owned()),
                starred: Some(true),
                due: Some(Some(date(2026, 11, 20))),
                ..TaskDraft::default()
            },
        )
        .unwrap();

    let changed = store.task(&TaskId("t1".to_owned())).unwrap().unwrap();
    assert_eq!(changed.title, "Oat milk");
    assert_eq!(changed.importance, Importance::High);
    assert_eq!(changed.due, Some(date(2026, 11, 20)));
    assert_eq!(changed.notes, "Semi-skimmed");
}

#[test]
fn completing_a_task_records_when_it_was_done_and_reopening_clears_it() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    store.upsert_tasks(&[task("t1", "l1")]).unwrap();
    let id = TaskId("t1".to_owned());

    store
        .change_task(
            &id,
            &TaskDraft {
                completed: Some(true),
                ..TaskDraft::default()
            },
        )
        .unwrap();
    let done = store.task(&id).unwrap().unwrap();
    assert_eq!(done.status, Status::Completed);
    assert!(done.completed_at.is_some());

    store
        .change_task(
            &id,
            &TaskDraft {
                completed: Some(false),
                ..TaskDraft::default()
            },
        )
        .unwrap();
    let open = store.task(&id).unwrap().unwrap();
    assert_eq!(open.status, Status::NotStarted);
    assert!(open.completed_at.is_none());
}

#[test]
fn a_task_can_be_cleared_of_its_due_date_and_reminder() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    let mut milk = task("t1", "l1");
    milk.due = Some(date(2026, 11, 20));
    milk.reminder = Some(Timestamp::from_second(1_700_000_000).unwrap());
    store.upsert_tasks(&[milk]).unwrap();
    let id = TaskId("t1".to_owned());

    store
        .change_task(
            &id,
            &TaskDraft {
                due: Some(None),
                reminder: Some(None),
                ..TaskDraft::default()
            },
        )
        .unwrap();

    let changed = store.task(&id).unwrap().unwrap();
    assert_eq!(changed.due, None);
    assert_eq!(changed.reminder, None);
}

#[test]
fn steps_come_back_in_the_order_they_were_added() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    store.upsert_tasks(&[task("t1", "l1")]).unwrap();
    let mut first = step("s1", "t1", false);
    first.created_at = Timestamp::from_second(10).unwrap();
    let mut second = step("s2", "t1", true);
    second.created_at = Timestamp::from_second(20).unwrap();

    store
        .put_steps(&TaskId("t1".to_owned()), &[second, first])
        .unwrap();

    let steps = store.steps(&TaskId("t1".to_owned())).unwrap();
    let ids: Vec<&str> = steps.iter().map(|step| step.id.0.as_str()).collect();
    assert_eq!(ids, ["s1", "s2"]);
}

#[test]
fn loading_steps_counts_them_on_the_task() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    store.upsert_tasks(&[task("t1", "l1")]).unwrap();

    store
        .put_steps(
            &TaskId("t1".to_owned()),
            &[step("s1", "t1", true), step("s2", "t1", false)],
        )
        .unwrap();

    let milk = store.task(&TaskId("t1".to_owned())).unwrap().unwrap();
    assert!(milk.steps_loaded);
    assert_eq!(milk.steps_done, 1);
    assert_eq!(milk.steps_total, 2);
}

#[test]
fn a_changed_task_forgets_the_steps_it_had_cached() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    store.upsert_tasks(&[task("t1", "l1")]).unwrap();
    store
        .put_steps(&TaskId("t1".to_owned()), &[step("s1", "t1", false)])
        .unwrap();
    let mut changed = task("t1", "l1");
    changed.modified_at = Timestamp::from_second(1_700_000_100).unwrap();

    store.upsert_tasks(&[changed]).unwrap();

    let milk = store.task(&TaskId("t1".to_owned())).unwrap().unwrap();
    assert!(!milk.steps_loaded);
}

#[test]
fn a_step_can_be_checked_renamed_and_deleted() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    store.upsert_tasks(&[task("t1", "l1")]).unwrap();
    store
        .put_steps(&TaskId("t1".to_owned()), &[step("s1", "t1", false)])
        .unwrap();

    let mut changed = step("s1", "t1", true);
    changed.name = "Check the fridge".to_owned();
    store.upsert_step(&changed).unwrap();

    let steps = store.steps(&TaskId("t1".to_owned())).unwrap();
    assert_eq!(steps[0].name, "Check the fridge");
    assert!(steps[0].checked);

    store.remove_step(&StepId("s1".to_owned())).unwrap();
    assert!(store.steps(&TaskId("t1".to_owned())).unwrap().is_empty());
}

#[test]
fn the_smart_views_show_important_planned_and_open_tasks() {
    let mut store = Store::open_in_memory().unwrap();
    store
        .upsert_lists(&[list("l1", "Groceries"), list("l2", "Work")])
        .unwrap();
    let mut starred = task("t1", "l1");
    starred.importance = Importance::High;
    let mut due = task("t2", "l2");
    due.due = Some(date(2026, 11, 20));
    due.created_at = Timestamp::from_second(1_700_000_100).unwrap();
    let mut done = task("t3", "l1");
    done.status = Status::Completed;
    store.upsert_tasks(&[starred, due, done]).unwrap();

    let important = store.important().unwrap();
    let planned = store.planned().unwrap();
    let open = store.all_open().unwrap();

    assert_eq!(important.len(), 1);
    assert_eq!(important[0].id.0, "t1");
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].id.0, "t2");
    let ids: Vec<&str> = open.iter().map(|task| task.id.0.as_str()).collect();
    assert_eq!(ids, ["t2", "t1"]);
}

#[test]
fn search_finds_tasks_by_title_or_notes_ignoring_case() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    let mut milk = task("t1", "l1");
    milk.title = "Milk".to_owned();
    let mut coffee = task("t2", "l1");
    coffee.title = "Coffee beans".to_owned();
    coffee.notes = "Ask about milk alternatives".to_owned();
    store.upsert_tasks(&[milk, coffee]).unwrap();

    let found = store.search("MILK", 10).unwrap();

    let ids: Vec<&str> = found.iter().map(|task| task.id.0.as_str()).collect();
    assert_eq!(ids, ["t1", "t2"]);
}

#[test]
fn search_treats_the_query_as_text_not_a_pattern() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    let mut milk = task("t1", "l1");
    milk.title = "100% milk".to_owned();
    store.upsert_tasks(&[milk]).unwrap();

    assert_eq!(store.search("100%", 10).unwrap().len(), 1);
    assert_eq!(store.search("%", 10).unwrap().len(), 1);
}

#[test]
fn tasks_can_be_removed_or_cleared_from_a_list() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();
    store
        .upsert_tasks(&[task("t1", "l1"), task("t2", "l1")])
        .unwrap();

    store.remove_tasks(&[TaskId("t1".to_owned())]).unwrap();
    assert!(store.task(&TaskId("t1".to_owned())).unwrap().is_none());

    store.clear_tasks_in(&ListId("l1".to_owned())).unwrap();
    assert!(store.task(&TaskId("t2".to_owned())).unwrap().is_none());
}

#[test]
fn delta_links_are_kept_per_list_and_once_for_the_list_of_lists() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_lists(&[list("l1", "Groceries")]).unwrap();

    assert_eq!(store.delta_link(&ListId("l1".to_owned())).unwrap(), None);
    assert_eq!(store.lists_delta_link().unwrap(), None);

    store
        .set_delta_link(&ListId("l1".to_owned()), Some("https://graph/tasks"))
        .unwrap();
    store
        .set_lists_delta_link(Some("https://graph/lists"))
        .unwrap();

    assert_eq!(
        store.delta_link(&ListId("l1".to_owned())).unwrap(),
        Some("https://graph/tasks".to_owned())
    );
    assert_eq!(
        store.lists_delta_link().unwrap(),
        Some("https://graph/lists".to_owned())
    );

    store
        .set_delta_link(&ListId("l1".to_owned()), None)
        .unwrap();
    store.set_lists_delta_link(None).unwrap();

    assert_eq!(store.delta_link(&ListId("l1".to_owned())).unwrap(), None);
    assert_eq!(store.lists_delta_link().unwrap(), None);
}
