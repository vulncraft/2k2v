use std::{fs::OpenOptions, path::PathBuf, process::exit};

use clap::Parser;

use kvnode::kvnode::Kvnode;
use tokio::signal::unix::{SignalKind, signal};
use tracing::info;

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

    let kvnode = Kvnode {
        http_addr: format!(
            "{}:{}",
            args.address.unwrap_or("0.0.0.0".into()),
            args.port.unwrap_or(3000)
        )
        .parse()
        .unwrap(),
        wal_path: PathBuf::from(&args.file),
    };
    // Ensure file exists
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(PathBuf::from(&args.file))
        .ok();

    let (shut_tx, shut_rx) = kvnode::shutdown::channel();

    // signal handler
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("sigterm");
        let mut sigint = signal(SignalKind::interrupt()).expect("sigint");
        tokio::select! {
            _ = sigterm.recv() => info!("received SIGTERM"),
            _ = sigint.recv() => info!("received SIGINT"),
        }
        shut_tx.trigger();
    });

    let result = Kvnode::start(kvnode, shut_rx).await;
    if result.is_err() {
        tracing::error!(message = "Error occured", err = ?result);
        exit(1);
    }
}
