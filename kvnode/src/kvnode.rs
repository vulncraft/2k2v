use std::{io, net::SocketAddr, path::PathBuf};

use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tracing::warn;

use crate::{
    actor::{ActorMessage, node_actor},
    node::NodeHttp,
    persistence::WalManager,
    shutdown,
};

#[derive(Error, Debug)]
enum RunTimeError {
    #[error("Error occured replaying wal")]
    Failed,
    #[error("SHA failed")]
    ShaError,
    #[error("IO error")]
    IoError(io::ErrorKind),
}
pub struct Kvnode {
    pub http_addr: SocketAddr,
    pub wal_path: PathBuf,
}
impl Kvnode {
    pub async fn start(node: Kvnode, shutdown: shutdown::ShutdownListener) -> anyhow::Result<()> {
        let (storage_actor, tx) = Kvnode::start_and_replay_storage_actor(&node.wal_path).await?;

        let http = NodeHttp {
            store_tx: tx,
            bind_address: node.http_addr,
        };
        let http_layer = tokio::spawn(http.run(shutdown));
        let _ = tokio::try_join!(http_layer, storage_actor);
        Ok(())
    }

    async fn start_and_replay_storage_actor(
        wal_path: &PathBuf,
    ) -> Result<(JoinHandle<()>, mpsc::Sender<ActorMessage>), RunTimeError> {
        let (tx, rx) = mpsc::channel::<ActorMessage>(32);
        let original_path = wal_path.clone();
        let backup_path = wal_path.parent().unwrap().join("kvnode_recovery.bin");
        let old_hash = sha256::try_digest(&wal_path).map_err(|_| RunTimeError::ShaError)?;
        // mv wal.bin wal.bin.bak
        std::fs::copy(wal_path, &backup_path).map_err(|e| RunTimeError::IoError(e.kind()))?;
        std::fs::remove_file(wal_path).map_err(|e| RunTimeError::IoError(e.kind()))?;
        let (rtx, rrx) = oneshot::channel();
        let storage_actor = tokio::spawn(node_actor(rx, original_path, rtx));
        rrx.await.expect("Fatal error in actor");

        let recovery_wal = WalManager::new(&backup_path).await;
        let replay_result = recovery_wal.replay(&tx).await;
        if replay_result.is_ok() {
            let new_hash = sha256::try_digest(&wal_path).map_err(|_| RunTimeError::ShaError)?;
            if new_hash == old_hash {
                std::fs::remove_file(&backup_path).map_err(|e| RunTimeError::IoError(e.kind()))?;
                return Ok((storage_actor, tx));
            }
        } else {
            warn!(message = "Could not replay WAL", err = ?replay_result);
        }
        // Error occured. rollback
        warn!("Rolling back recovery WAL");
        std::fs::copy(&backup_path, wal_path).map_err(|e| RunTimeError::IoError(e.kind()))?;
        std::fs::remove_file(&backup_path).map_err(|e| RunTimeError::IoError(e.kind()))?;
        Err(RunTimeError::Failed)
    }
}
