use anyhow::Error;
use rkyv::Archive;
use rkyv::rancor;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;

use tokio::sync::oneshot;
use tracing::info;

use crate::storage::KVStore;
use crate::storage::StorageInterface;

#[derive(Archive, rkyv::Serialize, Deserialize, Debug, PartialEq)]
pub struct PutRequest {
    pub key: String,
    pub value: String,
}

#[derive(Archive, rkyv::Serialize, Deserialize, Debug, PartialEq)]
pub enum StoreCommand {
    Get { key: String },
    Put { record: PutRequest },
    Delete { key: String },
}

pub type ActorMessage = (StoreCommand, Option<oneshot::Sender<Option<String>>>);

// Actor controls access to the internal local store by message passing
pub async fn node_actor(mut rx: mpsc::Receiver<ActorMessage>) {
    let mut store = KVStore::default();
    while let Some(msg) = rx.recv().await {
        info!(message = "Actor received:", message = ?msg);

        match msg {
            (StoreCommand::Get { key }, reply) if reply.is_some() => {
                if let Some(tx) = reply {
                    let _ = tx.send(store.get(&key));
                }
            }
            (cmd @ StoreCommand::Put { .. }, None) => {
                // Log
                let bytes = rkyv::to_bytes::<rancor::Error>(&cmd).unwrap();
                println!("{:?}", bytes);
                if let StoreCommand::Put {
                    record: PutRequest { key, value },
                } = cmd
                {
                    store.put(&key, &value);
                }
            }
            (cmd @ StoreCommand::Delete { .. }, None) => {
                // Log
                let bytes = rkyv::to_bytes::<rancor::Error>(&cmd).unwrap();
                println!("{:?}", bytes);
                if let StoreCommand::Delete { key } = cmd {
                    store.delete(&key);
                }
            }
            (msg, return_channel) => {
                panic!("illegal {:?} - {:#?}", msg, return_channel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_key(tx: &mpsc::Sender<ActorMessage>, key: String) -> Option<String> {
        // Check key not exist
        let (itx, irx) = oneshot::channel();
        let _ = tx.send((StoreCommand::Get { key }, Some(itx))).await;
        irx.await.unwrap()
    }

    #[tokio::test]
    async fn actor_fileops() {
        let (tx, rx) = mpsc::channel::<ActorMessage>(32);
        tokio::spawn(node_actor(rx));

        // Check key not exist
        assert!(get_key(&tx, "key".into()).await.is_none());

        // insert key
        let _ = tx
            .clone()
            .send((
                StoreCommand::Put {
                    record: PutRequest {
                        key: "key".into(),
                        value: "new_value".into(),
                    },
                },
                None,
            ))
            .await;

        // Check key exists
        let (itx, irx) = oneshot::channel();
        let _ = tx
            .send((StoreCommand::Get { key: "key".into() }, Some(itx)))
            .await;
        let response = irx.await.unwrap();
        assert_eq!(response, Some("new_value".into()));
        assert_eq!(get_key(&tx, "key".into()).await, Some("new_value".into()));

        // Remove key
        let _ = tx
            .clone()
            .send((StoreCommand::Delete { key: "key".into() }, None))
            .await;

        // Check key not exist
        assert!(get_key(&tx, "key".into()).await.is_none());
    }
}
