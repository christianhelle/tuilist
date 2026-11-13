use serde_json::json;
use tuilist::{
    auth::{AuthError, TokenSource},
    graph::GraphClient,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

struct Token;

impl TokenSource for Token {
    async fn access_token(&self) -> Result<String, AuthError> {
        Ok("access".to_owned())
    }

    async fn invalidate(&self) {}
}

async fn signed_in_as(user: serde_json::Value) -> Option<String> {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(query_param("$select", "mail,userPrincipalName"))
        .and(header("authorization", "Bearer access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user))
        .mount(&server)
        .await;

    GraphClient::new(server.uri(), Token)
        .address()
        .await
        .unwrap()
}

#[tokio::test]
async fn the_address_is_the_mailbox_address() {
    let address = signed_in_as(json!({
        "mail": "me@work.example",
        "userPrincipalName": "me@tenant.onmicrosoft.com"
    }))
    .await;

    assert_eq!(address.as_deref(), Some("me@work.example"));
}

#[tokio::test]
async fn without_a_mailbox_address_the_sign_in_name_stands_in() {
    let address = signed_in_as(json!({
        "mail": null,
        "userPrincipalName": "me@outlook.com"
    }))
    .await;

    assert_eq!(address.as_deref(), Some("me@outlook.com"));
}
