use std::path::PathBuf;

use anyhow::{self, Error};
use rkyv::rancor;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{Mutex, mpsc, oneshot},
};

use crate::actor::{ActorMessage, StoreCommand};

// Manages access to the WAL
#[allow(dead_code)]
pub struct WalManager {
    path: PathBuf,
    fd: Mutex<File>,
}

impl WalManager {
    pub async fn new(path: &PathBuf) -> Self {
        let fd = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .read(true)
            .open(&path)
            .await
            .unwrap();
        WalManager {
            path: path.clone(),
            fd: Mutex::new(fd),
        }
    }

    // Persist a command to wal
    pub async fn write(&self, cmd: &StoreCommand) {
        let bytes = rkyv::to_bytes::<rancor::Error>(cmd).unwrap();
        let mut guard = self.fd.lock().await;
        assert!(
            bytes.len() < u16::MAX as usize,
            "Wal message exceeded u16::max"
        );
        guard.write_u16(bytes.len() as u16).await.unwrap();
        guard.write_all(&bytes).await.unwrap();
        guard.flush().await.unwrap();
    }

    pub async fn replay(&self, tx: &mpsc::Sender<ActorMessage>) -> Result<(), Error> {
        // hold the lock for duration of replay
        let mut guard = self.fd.lock().await;
        guard.seek(std::io::SeekFrom::Start(0)).await?;
        loop {
            let len = guard.read_u16().await;
            if len.is_err() {
                break;
            }
            let len = len.unwrap() as usize;
            let mut buf = vec![0u8; len];
            // let mut aligned = rkyv::util::AlignedVec::with_capacity(len);
            // aligned.resize(len, 0);
            guard.read_exact(&mut buf).await?;
            if let Ok(cmd) = rkyv::from_bytes::<StoreCommand, rancor::Error>(&buf) {
                let (itx, irx) = oneshot::channel();
                tx.send((cmd, itx)).await?;
                let _ = irx.await;
            } else {
                return Err(anyhow::anyhow!("Invalid record format in WAL"));
            }
        }
        Ok(())
    }

    pub async fn sync(&self) {
        let _ = self.fd.lock().await.sync_all().await;
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

        let wal = WalManager::new(&wal_path).await;
        let mut operations = Vec::with_capacity(10);
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
            wal.write(op).await;
        }
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        // replay() forwards each command and then blocks on its oneshot reply,
        // so it must run concurrently with a consumer of `rx` that answers it.
        let replay = tokio::spawn(async move { wal.replay(&tx).await });

        let mut actual = Vec::new();
        while actual.len() < operations.len() {
            let (cmd, reply) = rx.recv().await.unwrap();
            let _ = reply.send(None);
            actual.push(cmd);
        }
        let _ = replay.await.unwrap();
        // Note GET is not ommited since we are not calling through node_actor

        assert_eq!(actual.len(), operations.len());
        for (r_actual, r_expected) in actual.iter().zip(operations) {
            assert_eq!(*r_actual, r_expected);
        }

        tmp.close().unwrap();
    }
}
