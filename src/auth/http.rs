/// Names tuilist in every request, so Microsoft can tell its traffic from an
/// unknown script's.
pub const USER_AGENT: &str = concat!(
    "tuilist/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/christianhelle/tuilist)"
);

/// The HTTP client every tuilist request goes through.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("a client with only a user agent set always builds")
}
