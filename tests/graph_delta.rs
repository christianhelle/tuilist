mod support;

use tuilist::model::{BackendError, ListId, TaskId, TodoBackend};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::test]
async fn an_expired_list_delta_link_asks_for_a_full_resync() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(410).set_body_json(
            serde_json::json!({ "error": { "code": "resyncRequired", "message": "gone" } }),
        ))
        .mount(&server)
        .await;
    let old = format!("{}/old", server.uri());

    let error = support::backend(&server)
        .lists_delta(Some(&old))
        .await
        .unwrap_err();

    assert_eq!(error, BackendError::DeltaExpired);
}

#[tokio::test]
async fn an_expired_task_delta_link_asks_for_a_full_resync() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(410).set_body_json(
            serde_json::json!({ "error": { "code": "resyncRequired", "message": "gone" } }),
        ))
        .mount(&server)
        .await;
    let old = format!("{}/old", server.uri());

    let error = support::backend(&server)
        .tasks_delta(&ListId("work".to_owned()), Some(&old))
        .await
        .unwrap_err();

    assert_eq!(error, BackendError::DeltaExpired);
}

#[tokio::test]
async fn a_missing_task_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/me/todo/lists/work/tasks/t1"))
        .respond_with(ResponseTemplate::new(404).set_body_json(
            serde_json::json!({ "error": { "code": "itemNotFound", "message": "gone" } }),
        ))
        .mount(&server)
        .await;

    let error = support::backend(&server)
        .delete_task(&ListId("work".to_owned()), &TaskId("t1".to_owned()))
        .await
        .unwrap_err();

    assert_eq!(error, BackendError::NotFound);
}
