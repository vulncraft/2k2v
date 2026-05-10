use tokio::sync::mpsc;

use kvnode::{
    actor::{StoreCommand, node_actor},
    node::NodeHttp,
};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    info!("init");
    let (tx, rx) = mpsc::channel::<StoreCommand>(32);
    tokio::spawn(node_actor(rx));
    let node = NodeHttp { store_tx: tx };

    node.run().await;
}
