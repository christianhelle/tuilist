//! Microsoft Graph: the REST calls and the model they map to.

mod backend;
mod client;
mod dto;

pub use backend::GraphBackend;
pub use client::{GRAPH_V1, GraphClient, GraphError, RetryPolicy};
