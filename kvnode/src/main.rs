use clap::Parser;
use tokio::sync::mpsc;

use kvnode::{
    actor::{ActorMessage, node_actor},
    node::NodeHttp,
};
use tracing::info;

#[derive(Parser)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    address: Option<String>,

    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let args = Args::parse();
    info!("init");

    let (tx, rx) = mpsc::channel::<ActorMessage>(32);
    tokio::spawn(node_actor(rx));
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
