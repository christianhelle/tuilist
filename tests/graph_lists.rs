mod support;

use serde_json::json;
use tuilist::model::{TodoBackend, WellKnown};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

#[tokio::test]
async fn lists_delta_follows_next_link_and_keeps_the_delta_link() {
    let server = MockServer::start().await;
    let more = format!("{}/more-lists", server.uri());
    let delta = format!("{}/lists-delta", server.uri());
    Mock::given(method("GET"))
        .and(path("/me/todo/lists/delta"))
        .and(header("authorization", "Bearer access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "id": "default",
                "displayName": "Tasks",
                "wellknownListName": "defaultList",
                "isOwner": true,
                "isShared": false
            }],
            "@odata.nextLink": more
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/more-lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "id": "work",
                "displayName": "Work",
                "wellknownListName": "none",
                "isOwner": false,
                "isShared": true
            }],
            "@odata.deltaLink": delta
        })))
        .mount(&server)
        .await;

    let page = support::backend(&server).lists_delta(None).await.unwrap();

    let names: Vec<&str> = page.upserted.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Tasks", "Work"]);
    assert_eq!(page.upserted[0].well_known, Some(WellKnown::DefaultList));
    assert_eq!(page.upserted[1].well_known, None);
    assert!(!page.upserted[1].is_owner);
    assert!(page.upserted[1].is_shared);
    assert_eq!(page.delta_link.as_deref(), Some(delta.as_str()));
}

#[tokio::test]
async fn a_deleted_list_is_reported_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/todo/lists/delta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [
                { "id": "gone", "@removed": { "reason": "deleted" } },
                { "id": "work", "displayName": "Work" }
            ]
        })))
        .mount(&server)
        .await;

    let page = support::backend(&server).lists_delta(None).await.unwrap();

    assert_eq!(page.removed.len(), 1);
    assert_eq!(page.removed[0].0, "gone");
    assert_eq!(page.upserted.len(), 1);
}

#[tokio::test]
async fn a_list_is_created_with_its_name() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/me/todo/lists"))
        .and(body_json(json!({ "displayName": "Errands" })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": "new",
            "displayName": "Errands",
            "wellknownListName": "none"
        })))
        .mount(&server)
        .await;

    let list = support::backend(&server)
        .create_list("Errands")
        .await
        .unwrap();

    assert_eq!(list.id.0, "new");
    assert_eq!(list.name, "Errands");
}

#[tokio::test]
async fn a_list_is_renamed_by_patching_its_name() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/me/todo/lists/work"))
        .and(body_json(json!({ "displayName": "Home" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "work",
            "displayName": "Home"
        })))
        .mount(&server)
        .await;

    support::backend(&server)
        .rename_list(&tuilist::model::ListId("work".to_owned()), "Home")
        .await
        .unwrap();
}

#[tokio::test]
async fn a_list_is_deleted_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/me/todo/lists/work"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    support::backend(&server)
        .delete_list(&tuilist::model::ListId("work".to_owned()))
        .await
        .unwrap();
}
