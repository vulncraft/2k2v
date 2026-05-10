use tokio::sync::mpsc;

use tokio::sync::oneshot;

use crate::storage::KVStore;
use crate::storage::StorageInterface;

pub enum StoreCommand {
    Get {
        key: String,
        reply: oneshot::Sender<Option<String>>,
    },
    Put {
        key: String,
        value: String,
    },
    Delete {
        key: String,
    },
}

// Actor controls access to the internal local store by message passing
pub async fn node_actor(mut rx: mpsc::Receiver<StoreCommand>) {
    let mut store = KVStore::default();
    while let Some(cmd) = rx.recv().await {
        match cmd {
            StoreCommand::Get { key, reply } => {
                let _ = reply.send(store.get(&key));
            }
            StoreCommand::Put { key, value } => {
                store.put(&key, &value);
            }
            StoreCommand::Delete { key } => {
                store.delete(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_key(tx: &mpsc::Sender<StoreCommand>, key: String) -> Option<String> {
        // Check key not exist
        let (itx, irx) = oneshot::channel();
        let _ = tx
            .send(StoreCommand::Get {
                key: "key".into(),
                reply: itx,
            })
            .await;
        irx.await.unwrap()
    }

    #[tokio::test]
    async fn actor_fileops() {
        let (tx, rx) = mpsc::channel::<StoreCommand>(32);
        tokio::spawn(node_actor(rx));

        // Check key not exist
        assert!(get_key(&tx, "key".into()).await.is_none());

        // insert key
        let _ = tx
            .clone()
            .send(StoreCommand::Put {
                key: "key".into(),
                value: "new_value".into(),
            })
            .await;

        // Check key exists
        let (itx, irx) = oneshot::channel();
        let _ = tx
            .send(StoreCommand::Get {
                key: "key".into(),
                reply: itx,
            })
            .await;
        let response = irx.await.unwrap();
        assert_eq!(response, Some("new_value".into()));
        assert_eq!(get_key(&tx, "key".into()).await, Some("new_value".into()));

        // Remove key
        let _ = tx
            .clone()
            .send(StoreCommand::Delete { key: "key".into() })
            .await;

        // Check key not exist
        assert!(get_key(&tx, "key".into()).await.is_none());
    }
}
