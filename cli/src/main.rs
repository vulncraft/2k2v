use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct PutRequest {
    key: String,
    value: String,
}

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    op: Operations,

    #[arg(short)]
    target: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Operations {
    Get {
        #[arg(short)]
        key: String,
    },
    Put {
        #[arg(short)]
        key: String,
        #[arg(short)]
        value: String,
    },
    Delete {
        #[arg(short)]
        key: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Args::parse();
    let host = cli.target.unwrap_or("http://localhost:3000".into());

    match cli.op {
        Operations::Get { key } => do_get(host, key).await,
        Operations::Put { key, value } => do_put(host, key, value).await,
        Operations::Delete { key } => do_delete(host, key).await,
    }
}

async fn do_get(host: String, key: String) {
    let url = format!("{}/key/{}", host, key);
    let response = reqwest::get(url).await.unwrap().text().await;
    println!("{:#?}", response);
}
async fn do_put(host: String, key: String, value: String) {
    let client = reqwest::Client::new();
    let url = format!("{}/key", host);
    let body = PutRequest { key, value };
    let response = client.put(url).json(&body).send().await;
    println!("{:#?}", response);
}

async fn do_delete(host: String, key: String) {
    let client = reqwest::Client::new();
    let url = format!("{}/key/{}", host, key);
    let response = client.delete(url).send().await;
    println!("{:#?}", response);
}
