#![allow(unused)]

use std::{
    collections::HashMap,
    default,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
use tower_http::trace::TraceLayer;
use tracing::{info, instrument};

use crate::actor::{self, ActorMessage, StoreCommand};

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
    pub store_tx: mpsc::Sender<ActorMessage>,
    pub bind_address: SocketAddr,
}

impl NodeHttp {
    fn router(&self) -> Router {
        Router::new()
            .route("/key/{key}", get(NodeHttp::handle_get_val))
            .route("/key", put(NodeHttp::handle_put_val))
            .route("/key/{key}", delete(NodeHttp::handle_del_val))
            .route("/health", get(NodeHttp::handle_health))
            .route("/ready", get(NodeHttp::handle_ready))
            .with_state(self.store_tx.clone())
            .layer(TraceLayer::new_for_http())
    }

    // Run the node
    pub async fn run(self) {
        let app = self.router();
        let listener = tokio::net::TcpListener::bind(&self.bind_address)
            .await
            .expect("Failed to bind port");
        axum::serve(listener, app).await;
    }

    async fn handle_health() -> StatusCode {
        StatusCode::OK
    }

    // Try getting a key from the KV store
    // if it works then all is initialized
    async fn handle_ready(State(tx): State<mpsc::Sender<ActorMessage>>) -> StatusCode {
        let (itx, irx) = oneshot::channel();
        let _ = tx.send((StoreCommand::NoOp, itx)).await;
        match timeout(Duration::from_secs(1), irx).await {
            Ok(Ok(_)) => StatusCode::OK,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    async fn handle_del_val(
        State(tx): State<mpsc::Sender<ActorMessage>>,
        Path(key): Path<String>,
    ) -> StatusCode {
        info!(message = "Deleted key", key = &key);
        let (itx, irx) = oneshot::channel();
        let _ = tx.send((StoreCommand::Delete { key }, itx)).await;
        match irx.await {
            Ok(_) => (StatusCode::OK),
            _ => (StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn handle_put_val(
        State(tx): State<mpsc::Sender<ActorMessage>>,
        Json(body): Json<actor::PutRequest>,
    ) -> (StatusCode) {
        // info!(message = "Updated key", key = &key, new_value = val);
        let (itx, irx) = oneshot::channel();
        let _ = tx
            .send((
                StoreCommand::Put {
                    record: actor::PutRequest {
                        key: body.key,
                        value: body.value,
                    },
                },
                itx,
            ))
            .await;

        match irx.await {
            Ok(_) => (StatusCode::OK),
            _ => (StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    // Get a value from the KV store
    async fn handle_get_val(
        State(tx): State<mpsc::Sender<ActorMessage>>,
        Path(key): Path<String>,
    ) -> (StatusCode, Json<StoredValue>) {
        info!(message = "get key", key = key);

        let (itx, irx) = oneshot::channel();
        let _ = tx.send((StoreCommand::Get { key: key.clone() }, itx)).await;
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
    use std::{net::Ipv4Addr, path::PathBuf};

    use crate::actor::node_actor;

    use super::*;

    use axum_test::TestServer;
    use serde_json::json;
    #[tokio::test]
    async fn test_ops() {
        let (tx, rx) = mpsc::channel::<ActorMessage>(32);
        let tmp_wal = PathBuf::from("/dev/null");
        let (rtx, rrx) = oneshot::channel();
        tokio::spawn(node_actor(rx, tmp_wal, rtx));
        rrx.await.unwrap();
        let node = NodeHttp {
            store_tx: tx,
            bind_address: "0.0.0.0:3000".parse().unwrap(),
        };
        let router = node.router();
        let server = TestServer::new(router);
        let response = server
            .get("/key/unknown_key")
            .await
            .assert_status(StatusCode::NOT_FOUND);
        let response = server
            .put("/key")
            .json(&json!({"key": "new_key", "value": "new_value"}))
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
