#![allow(unused)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum NodeError {
    #[error("Key did not exist in KV store")]
    KeyDidNotExist(String),
}

#[derive(Serialize)]
struct StoredValue {
    key: String,
    value: Option<String>,
    error: Option<NodeError>,
}

pub struct Node {}

#[derive(Default)]
struct AppState {
    kv: HashMap<String, String>,
}
impl AppState {
    pub fn get_val(&self, key: &String) -> anyhow::Result<String, NodeError> {
        if let Some(val) = self.kv.get(key) {
            return Ok(val.clone());
        }
        Err(NodeError::KeyDidNotExist(key.clone()))
    }
}

impl Node {
    // Run the node
    pub async fn run(self) {
        let shared_state = Arc::new(Mutex::new(AppState::default()));
        shared_state
            .lock()
            .unwrap()
            .kv
            .insert("test".into(), "value".into());
        let app = Router::new()
            .route("/key/{key}", get(Node::handle_get_val))
            .with_state(shared_state);
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
            .await
            .expect("Failed to bind port");
        axum::serve(listener, app).await;
    }

    // Get a value from the KV store
    async fn handle_get_val(
        State(state): State<Arc<Mutex<AppState>>>,
        Path(key): Path<String>,
    ) -> Json<StoredValue> {
        let val = state.lock().unwrap().get_val(&key);
        if let Ok(val) = val {
            Json(StoredValue {
                key,
                value: Some(val),
                error: None,
            })
        } else {
            Json(StoredValue {
                key,
                value: None,
                error: val.err(),
            })
        }
    }
}
