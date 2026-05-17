use std::path::PathBuf;

use rkyv::Archive;
use tokio::sync::mpsc;

use tokio::sync::oneshot;
use tracing::info;

use crate::persistence::WalManager;
use crate::storage::KVStore;
use crate::storage::StorageInterface;

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, serde::Deserialize, Debug, PartialEq)]
pub struct PutRequest {
    pub key: String,
    pub value: String,
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, serde::Deserialize, Debug, PartialEq)]
pub enum StoreCommand {
    Get { key: String },
    Put { record: PutRequest },
    Delete { key: String },
}

pub type ActorMessage = (StoreCommand, oneshot::Sender<Option<String>>);

// Actor controls access to the internal local store by message passing
pub async fn node_actor(mut rx: mpsc::Receiver<ActorMessage>, wal_path: PathBuf) {
    let mut store = KVStore::default();
    let wal = WalManager::new(&wal_path).await;
    while let Some(msg) = rx.recv().await {
        info!(message = "Actor received:", message = ?msg);

        match msg {
            (StoreCommand::Get { key }, reply) => {
                let _ = reply.send(store.get(&key));
            }
            (cmd @ StoreCommand::Put { .. }, reply) => {
                // Log
                wal.write(&cmd).await;
                if let StoreCommand::Put {
                    record: PutRequest { key, value },
                } = cmd
                {
                    store.put(&key, &value);
                    reply.send(None).unwrap();
                }
            }
            (cmd @ StoreCommand::Delete { .. }, reply) => {
                // Log
                wal.write(&cmd).await;
                if let StoreCommand::Delete { key } = cmd {
                    store.delete(&key);
                    reply.send(None).unwrap();
                }
            }
        }
    }

    // wal.replay().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_key(tx: &mpsc::Sender<ActorMessage>, key: String) -> Option<String> {
        // Check key not exist
        let (itx, irx) = oneshot::channel();
        let _ = tx.send((StoreCommand::Get { key }, itx)).await;
        irx.await.unwrap()
    }

    #[tokio::test]
    async fn actor_fileops() {
        let (tx, rx) = mpsc::channel::<ActorMessage>(32);
        let tmp_wal = PathBuf::from("/dev/null");
        let (itx, irx) = oneshot::channel();
        tokio::spawn(node_actor(rx, tmp_wal));

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
                itx,
            ))
            .await;

        let response = irx.await.unwrap();
        assert_eq!(None, response);

        // Check key exists
        let (itx, irx) = oneshot::channel();
        let _ = tx
            .send((StoreCommand::Get { key: "key".into() }, itx))
            .await;
        let response = irx.await.unwrap();
        assert_eq!(response, Some("new_value".into()));
        assert_eq!(get_key(&tx, "key".into()).await, Some("new_value".into()));

        let (itx, irx) = oneshot::channel();
        // Remove key
        let _ = tx
            .clone()
            .send((StoreCommand::Delete { key: "key".into() }, itx))
            .await;

        let response = irx.await.unwrap();
        assert_eq!(None, response);
        // Check key not exist
        assert!(get_key(&tx, "key".into()).await.is_none());
    }
}
