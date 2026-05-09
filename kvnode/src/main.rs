use kvnode::node::Node;

#[tokio::main]
async fn main() {
    let node = Node {};
    node.run().await;
    println!("Hello, world!");
}
