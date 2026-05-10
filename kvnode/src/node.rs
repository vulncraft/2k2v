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
use tokio::sync::{mpsc, oneshot};
use tower_http::trace::TraceLayer;
use tracing::{info, instrument};

use crate::actor::{self, StoreCommand};

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

pub struct NodeHttp {
    pub store_tx: mpsc::Sender<actor::StoreCommand>,
}

impl NodeHttp {
    fn router(self) -> Router {
        Router::new()
            .route("/key/{key}", get(NodeHttp::handle_get_val))
            .route("/key/{key}", put(NodeHttp::handle_put_val))
            .route("/key/{key}", delete(NodeHttp::handle_del_val))
            .with_state(self.store_tx)
            .layer(TraceLayer::new_for_http())
    }

    // Run the node
    pub async fn run(self) {
        let node = NodeHttp {
            store_tx: self.store_tx,
        };
        let app = node.router();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
            .await
            .expect("Failed to bind port");
        axum::serve(listener, app).await;
    }

    async fn handle_del_val(
        State(tx): State<mpsc::Sender<StoreCommand>>,
        Path(key): Path<String>,
    ) -> StatusCode {
        info!(message = "Deleted key", key = &key);
        let _ = tx.clone().send(StoreCommand::Delete { key }).await;
        StatusCode::OK
    }

    async fn handle_put_val(
        State(tx): State<mpsc::Sender<StoreCommand>>,
        Path(key): Path<String>,
        Query(params): Query<HashMap<String, String>>,
    ) -> (StatusCode) {
        if let Some(val) = params.get("value") {
            info!(message = "Updated key", key = &key, new_value = val);
            let _ = tx
                .clone()
                .send(StoreCommand::Put {
                    key,
                    value: val.clone(),
                })
                .await;
        } else {
            return StatusCode::BAD_REQUEST;
        }
        (StatusCode::OK)
    }

    // Get a value from the KV store
    async fn handle_get_val(
        State(tx): State<mpsc::Sender<StoreCommand>>,
        Path(key): Path<String>,
    ) -> (StatusCode, Json<StoredValue>) {
        info!(message = "get key", key = key);

        let (itx, irx) = oneshot::channel();
        let _ = tx
            .send(StoreCommand::Get {
                key: key.clone(),
                reply: itx,
            })
            .await;
        if let Some(val) = irx.await.unwrap() {
            (
                StatusCode::OK,
                Json(StoredValue {
                    key,
                    value: Some(val),
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
    use crate::actor::node_actor;

    use super::*;

    use axum_test::TestServer;
    use serde_json::json;
    #[tokio::test]
    async fn test_ops() {
        let (tx, rx) = mpsc::channel::<StoreCommand>(32);
        tokio::spawn(node_actor(rx));
        let node = NodeHttp { store_tx: tx };
        let router = node.router();
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
