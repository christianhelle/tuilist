use reqwest::{Method, StatusCode};
use serde::{Deserialize, de::DeserializeOwned};

use crate::auth::{AuthError, TokenSource};

pub const GRAPH_V1: &str = "https://graph.microsoft.com/v1.0";

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("unexpected response from {url}: {source}")]
    Decode {
        url: String,
        source: serde_json::Error,
    },
    #[error("Graph answered {status}: {code}: {message}")]
    Status {
        status: StatusCode,
        code: String,
        message: String,
    },
}

/// Talks to Graph with whatever token source the caller has.
pub struct GraphClient<T> {
    http: reqwest::Client,
    base_url: String,
    tokens: T,
}

impl<T: TokenSource> GraphClient<T> {
    pub fn new(base_url: impl Into<String>, tokens: T) -> Self {
        Self {
            http: crate::auth::http_client(),
            base_url: base_url.into(),
            tokens,
        }
    }

    /// The signed-in account's address: its mailbox, or its sign-in name
    /// when it has no mailbox of its own.
    pub async fn address(&self) -> Result<Option<String>, GraphError> {
        #[derive(Deserialize)]
        struct Me {
            mail: Option<String>,
            #[serde(rename = "userPrincipalName")]
            user_principal_name: Option<String>,
        }

        let me: Me = self.get("/me?$select=mail,userPrincipalName").await?;
        Ok(me.mail.or(me.user_principal_name))
    }

    /// GETs `path`, relative to the base URL.
    pub async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R, GraphError> {
        let url = self.url(path);
        let response = self
            .http
            .request(Method::GET, &url)
            .bearer_auth(self.tokens.access_token().await?)
            .send()
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            self.tokens.invalidate().await;
        }
        decode(response, url).await
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_owned();
        }
        format!("{}{path}", self.base_url.trim_end_matches('/'))
    }
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorPayload,
}

#[derive(Deserialize)]
struct ErrorPayload {
    code: String,
    message: String,
}

async fn decode<T: DeserializeOwned>(
    response: reqwest::Response,
    url: String,
) -> Result<T, GraphError> {
    let status = response.status();
    let text = response.text().await?;
    if status.is_success() {
        return serde_json::from_str(&text).map_err(|source| GraphError::Decode { url, source });
    }
    match serde_json::from_str::<ErrorBody>(&text) {
        Ok(body) => Err(GraphError::Status {
            status,
            code: body.error.code,
            message: body.error.message,
        }),
        Err(source) => Err(GraphError::Decode { url, source }),
    }
}
