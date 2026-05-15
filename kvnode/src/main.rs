use std::{fs::OpenOptions, path::PathBuf, process::exit};

use clap::Parser;
use tokio::sync::mpsc;

use kvnode::{
    actor::{ActorMessage, node_actor},
    node::NodeHttp,
    persistence::WalManager,
};
use tracing::{error, info, info_span};

#[derive(Parser)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    file: String,

    #[arg(short, long)]
    address: Option<String>,

    #[arg(short, long)]
    port: Option<u16>,

    #[arg(short, long)]
    recover: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let args = Args::parse();
    info!("init");

    let (tx, rx) = mpsc::channel::<ActorMessage>(32);
    let wal = args.file;
    let wal_path = PathBuf::from(&wal);
    {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&wal_path)
            .ok();
    }

    info!("Recovering wal.bin");
    let hash = sha256::try_digest(&wal_path).unwrap();

    let _old_wal_path = wal_path.clone();
    let old_wal_path = PathBuf::from(format!("{}_bak", &wal));
    std::fs::copy(&wal_path, &old_wal_path).unwrap();
    std::fs::remove_file(&wal_path).unwrap();

    tokio::spawn(node_actor(rx, wal_path));

    let recovery_wal = WalManager::new(old_wal_path.to_path_buf()).await;
    {
        info_span!("replay_");
        recovery_wal.replay(&tx).await;
    }

    let new_hash = sha256::try_digest(&old_wal_path).unwrap();
    if hash != new_hash {
        error!("Failed to replay!");
        exit(1);
    } else {
        std::fs::remove_file(old_wal_path).unwrap();
        info!("Replay succeeded");
    }

    let node = NodeHttp {
        store_tx: tx,
        bind_address: format!(
            "{}:{}",
            args.address.unwrap_or("localhost".into()),
            args.port.unwrap_or(3000)
        ),
    };

    node.run().await;
}
