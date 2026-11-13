mod support;

use serde_json::json;
use tuilist::model::{ListId, StepId, TaskId, TodoBackend};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, body_partial_json, method, path},
};

fn list() -> ListId {
    ListId("work".to_owned())
}

fn task() -> TaskId {
    TaskId("t1".to_owned())
}

#[tokio::test]
async fn steps_page_through_and_keep_their_order() {
    let server = MockServer::start().await;
    let more = format!("{}/more-steps", server.uri());
    Mock::given(method("GET"))
        .and(path("/me/todo/lists/work/tasks/t1/checklistItems"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "id": "s1",
                "displayName": "Buy stamps",
                "isChecked": false,
                "createdDateTime": "2026-11-10T09:00:00Z"
            }],
            "@odata.nextLink": more
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/more-steps"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "id": "s2",
                "displayName": "Post the letter",
                "isChecked": true
            }]
        })))
        .mount(&server)
        .await;

    let steps = support::backend(&server)
        .steps(&list(), &task())
        .await
        .unwrap();

    let names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Buy stamps", "Post the letter"]);
    assert!(!steps[0].checked);
    assert!(steps[1].checked);
    assert_eq!(steps[0].task, task());
}

#[tokio::test]
async fn a_step_is_created_with_its_name() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/me/todo/lists/work/tasks/t1/checklistItems"))
        .and(body_json(json!({ "displayName": "Buy stamps" })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": "s1",
            "displayName": "Buy stamps",
            "isChecked": false
        })))
        .mount(&server)
        .await;

    let step = support::backend(&server)
        .create_step(&list(), &task(), "Buy stamps")
        .await
        .unwrap();

    assert_eq!(step.id.0, "s1");
    assert_eq!(step.name, "Buy stamps");
}

#[tokio::test]
async fn a_step_checked_and_renamed_sends_both_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/me/todo/lists/work/tasks/t1/checklistItems/s1"))
        .and(body_partial_json(json!({
            "displayName": "Post it",
            "isChecked": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "s1",
            "displayName": "Post it",
            "isChecked": true
        })))
        .mount(&server)
        .await;

    support::backend(&server)
        .update_step(
            &list(),
            &task(),
            &StepId("s1".to_owned()),
            Some("Post it"),
            Some(true),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_step_is_deleted_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/me/todo/lists/work/tasks/t1/checklistItems/s1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    support::backend(&server)
        .delete_step(&list(), &task(), &StepId("s1".to_owned()))
        .await
        .unwrap();
}
