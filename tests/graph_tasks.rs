mod support;

use serde_json::json;
use tuilist::model::{Importance, ListId, Status, TaskDraft, TaskId, TodoBackend};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, header, method, path},
};

fn list() -> ListId {
    ListId("work".to_owned())
}

#[tokio::test]
async fn tasks_delta_pages_through_and_maps_the_task() {
    let server = MockServer::start().await;
    let more = format!("{}/more-tasks", server.uri());
    let delta = format!("{}/tasks-delta", server.uri());
    Mock::given(method("GET"))
        .and(path("/me/todo/lists/work/tasks/delta"))
        .and(header("prefer", "odata.maxpagesize=50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "id": "t1",
                "title": "Send the report",
                "status": "inProgress",
                "importance": "high",
                "createdDateTime": "2026-11-10T09:00:00Z",
                "dueDateTime": {
                    "dateTime": "2026-11-20T00:00:00.0000000",
                    "timeZone": "UTC"
                }
            }],
            "@odata.nextLink": more
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/more-tasks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "id": "t2",
                "title": "Buy milk",
                "status": "completed",
                "completedDateTime": {
                    "dateTime": "2026-11-12T08:30:00",
                    "timeZone": "UTC"
                },
                "createdDateTime": "2026-11-09T09:00:00Z"
            }],
            "@odata.deltaLink": delta
        })))
        .mount(&server)
        .await;

    let page = support::backend(&server)
        .tasks_delta(&list(), None)
        .await
        .unwrap();

    assert_eq!(page.upserted.len(), 2);
    let report = &page.upserted[0];
    assert_eq!(report.title, "Send the report");
    assert_eq!(report.status, Status::InProgress);
    assert_eq!(report.importance, Importance::High);
    assert_eq!(report.due.as_ref().unwrap().to_string(), "2026-11-20");
    assert_eq!(page.upserted[1].status, Status::Completed);
    assert_eq!(page.delta_link.as_deref(), Some(delta.as_str()));
}

#[tokio::test]
async fn a_deleted_task_is_reported_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/todo/lists/work/tasks/delta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{ "id": "gone", "@removed": { "reason": "deleted" } }]
        })))
        .mount(&server)
        .await;

    let page = support::backend(&server)
        .tasks_delta(&list(), None)
        .await
        .unwrap();

    assert_eq!(page.removed, vec![TaskId("gone".to_owned())]);
}

#[tokio::test]
async fn a_task_is_created_with_its_draft() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/me/todo/lists/work/tasks"))
        .and(body_partial_json(json!({
            "title": "Buy milk",
            "importance": "high",
            "dueDateTime": {
                "dateTime": "2026-11-20T00:00:00",
                "timeZone": "UTC"
            }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": "t1",
            "title": "Buy milk",
            "importance": "high",
            "createdDateTime": "2026-11-13T09:00:00Z"
        })))
        .mount(&server)
        .await;

    let task = support::backend(&server)
        .create_task(
            &list(),
            &TaskDraft {
                title: Some("Buy milk".to_owned()),
                starred: Some(true),
                due: Some(Some(jiff::civil::date(2026, 11, 20))),
                ..TaskDraft::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(task.id.0, "t1");
    assert_eq!(task.title, "Buy milk");
    assert_eq!(task.importance, Importance::High);
}

#[tokio::test]
async fn an_update_sends_only_what_changed() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/me/todo/lists/work/tasks/t1"))
        .and(body_partial_json(json!({
            "importance": "normal",
            "status": "completed"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "t1",
            "title": "Buy milk",
            "status": "completed",
            "importance": "normal",
            "completedDateTime": {
                "dateTime": "2026-11-13T09:00:00",
                "timeZone": "UTC"
            },
            "createdDateTime": "2026-11-10T09:00:00Z"
        })))
        .mount(&server)
        .await;

    let task = support::backend(&server)
        .update_task(
            &list(),
            &TaskId("t1".to_owned()),
            &TaskDraft {
                starred: Some(false),
                completed: Some(true),
                ..TaskDraft::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(task.status, Status::Completed);
    assert!(task.completed_at.is_some());
}

#[tokio::test]
async fn a_task_is_deleted_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/me/todo/lists/work/tasks/t1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    support::backend(&server)
        .delete_task(&list(), &TaskId("t1".to_owned()))
        .await
        .unwrap();
}
