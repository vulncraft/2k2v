use std::path::PathBuf;

use rkyv::rancor;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{Mutex, mpsc},
};

use crate::actor::{ActorMessage, StoreCommand};

// Manages access to the WAL
#[allow(dead_code)]
pub struct WalManager {
    path: PathBuf,
    fd: Mutex<File>,
}

impl WalManager {
    pub async fn new(path: PathBuf) -> Self {
        let fd = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .read(true)
            .open(&path)
            .await
            .unwrap();
        WalManager {
            path,
            fd: Mutex::new(fd),
        }
    }

    // Persist a command to wal
    pub async fn write(&self, cmd: &StoreCommand) {
        let bytes = rkyv::to_bytes::<rancor::Error>(cmd).unwrap();
        let mut guard = self.fd.lock().await;
        guard.write_u64(bytes.len() as u64).await.unwrap();
        guard.write_all(&bytes).await.unwrap();
        guard.flush().await.unwrap();
    }

    pub async fn replay(&self, tx: &mpsc::Sender<ActorMessage>) {
        let mut guard = self.fd.lock().await;
        guard.seek(std::io::SeekFrom::Start(0)).await.unwrap();
        loop {
            let len = guard.read_u64().await;
            if len.is_err() {
                break;
            }
            let len = len.unwrap() as usize;
            let mut buf = vec![0u8; len];
            // let mut aligned = rkyv::util::AlignedVec::with_capacity(len);
            // aligned.resize(len, 0);
            guard.read_exact(&mut buf).await.unwrap();
            let cmd = rkyv::from_bytes::<StoreCommand, rancor::Error>(&buf).unwrap();
            tx.send((cmd, None)).await.unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_wal() {
        let tmp = TempDir::new("kvnodepersistence").unwrap();
        let wal_path = tmp.path().join("wal.bin");

        let wal = WalManager::new(wal_path).await;
        let mut operations = Vec::new();
        operations.push(StoreCommand::Get { key: "key".into() });
        operations.push(StoreCommand::Put {
            record: crate::actor::PutRequest {
                key: "key".into(),
                value: "a value".into(),
            },
        });
        operations.push(StoreCommand::Delete { key: "key".into() });
        operations.push(StoreCommand::Put {
            record: crate::actor::PutRequest {
                key: "key2".into(),
                value: "a value".into(),
            },
        });
        operations.push(StoreCommand::Get { key: "key".into() });
        operations.push(StoreCommand::Get { key: "key".into() });

        for op in &operations {
            wal.write(&op).await;
        }
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        wal.replay(&tx).await;
        let mut actual = Vec::new();

        let cmd = rx.recv_many(&mut actual, 10).await;
        // Note GET is not ommited since we are not calling through node_actor

        assert_eq!(actual.len(), operations.len());
        for (r_actual, r_expected) in actual.iter().zip(operations) {
            assert_eq!(r_actual.0, r_expected);
        }

        tmp.close().unwrap();
    }
}
