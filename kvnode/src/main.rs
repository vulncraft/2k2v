use tokio::sync::mpsc;

use kvnode::{
    actor::{StoreCommand, node_actor},
    node::NodeHttp,
};
use tracing::{Level, event, info, span};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    info!("init");
    let (tx, rx) = mpsc::channel::<StoreCommand>(32);
    tokio::spawn(node_actor(rx));
    let node = NodeHttp { store_tx: tx };

    println!("Hello, world!");
    node.run().await;
}
