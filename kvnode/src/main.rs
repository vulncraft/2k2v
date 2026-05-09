use kvnode::node::Node;
use tracing::{Level, event, info, span};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    info!("init");
    let node = Node {};
    node.run().await;
    println!("Hello, world!");
}
