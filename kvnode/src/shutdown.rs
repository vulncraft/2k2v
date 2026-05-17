use tokio::sync::watch;
use tracing::info;

pub fn channel() -> (ShutdownTrigger, ShutdownListener) {
    let (tx, rx) = watch::channel(false);
    (ShutdownTrigger(tx), ShutdownListener(rx))
}

pub struct ShutdownTrigger(watch::Sender<bool>);

impl ShutdownTrigger {
    pub fn trigger(self) {
        let _ = self.0.send(true);
        info!("Shutdown initiated");
    }
}

pub struct ShutdownListener(watch::Receiver<bool>);
impl ShutdownListener {
    pub async fn wait(mut self) {
        let _ = self.0.wait_for(|&v| v).await;
    }
    pub fn clone_listener(&self) -> Self {
        ShutdownListener(self.0.clone())
    }
}
