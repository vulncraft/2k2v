#![allow(unused)]

use std::{
    collections::HashMap,
    default,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde::Serialize;
use thiserror::Error;
use tower_http::trace::TraceLayer;
use tracing::{info, instrument};

#[derive(Error, Debug, Serialize)]
pub enum NodeError {
    #[error("Key did not exist in KV store")]
    KeyDidNotExist(String),
}

#[derive(Serialize)]
struct StoredValue {
    key: String,
    value: Option<String>,
}

pub struct Node {}

impl Node {
    fn router() -> Router {
        let shared_state = Arc::new(Mutex::new(HashMap::new()));
        shared_state
            .lock()
            .unwrap()
            .insert("test".into(), "value".into());
        Router::new()
            .route("/key/{key}", get(Node::handle_get_val))
            .route("/key/{key}", put(Node::handle_put_val))
            .route("/key/{key}", delete(Node::handle_del_val))
            .with_state(shared_state)
            .layer(TraceLayer::new_for_http())
    }

    // Run the node
    pub async fn run(self) {
        let app = Node::router();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
            .await
            .expect("Failed to bind port");
        axum::serve(listener, app).await;
    }

    async fn handle_del_val(
        State(state): State<Arc<Mutex<HashMap<String, String>>>>,
        Path(key): Path<String>,
    ) -> StatusCode {
        state.lock().unwrap().remove(&key);
        info!(message = "Deleted key", key = key);
        StatusCode::OK
    }

    async fn handle_put_val(
        State(state): State<Arc<Mutex<HashMap<String, String>>>>,
        Path(key): Path<String>,
        Query(params): Query<HashMap<String, String>>,
    ) -> (StatusCode) {
        if let Some(val) = params.get("value") {
            state
                .lock()
                .unwrap()
                .entry(key.clone())
                .insert_entry(val.into());
            info!(message = "Updated key", key = key, new_value = val);
        } else {
            return StatusCode::BAD_REQUEST;
        }
        (StatusCode::OK)
    }

    // Get a value from the KV store
    async fn handle_get_val(
        State(state): State<Arc<Mutex<HashMap<String, String>>>>,
        Path(key): Path<String>,
    ) -> (StatusCode, Json<StoredValue>) {
        info!(message = "get key", key = key);
        if let Some(val) = state.lock().unwrap().get(&key) {
            (
                StatusCode::OK,
                Json(StoredValue {
                    key,
                    value: Some(val.clone()),
                }),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(StoredValue { key, value: None }),
            )
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use axum_test::TestServer;
    use serde_json::json;
    #[tokio::test]
    async fn test_ops() {
        let router = Node::router();
        let server = TestServer::new(router);
        let response = server
            .get("/key/unknown_key")
            .await
            .assert_status(StatusCode::NOT_FOUND);
        let response = server
            .put("/key/new_key?value=new_value")
            .await
            .assert_status(StatusCode::OK);
        let response = server
            .get("/key/new_key")
            .await
            .assert_status(StatusCode::OK)
            .assert_json(&json!({"key": "new_key", "value": "new_value"}));

        let response = server
            .delete("/key/new_key")
            .await
            .assert_status(StatusCode::OK);
        let response = server
            .get("/key/new_key")
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }
}
