use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::watch;

static RELOAD_GEN: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct ReloadHandle {
    tx: watch::Sender<()>,
}

impl ReloadHandle {
    pub fn new() -> Self {
        let (tx, _) = watch::channel(());
        Self { tx }
    }

    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.tx.subscribe()
    }

    pub fn notify_reload(&self) {
        let _ = self.tx.send(());
        let c = RELOAD_GEN.fetch_add(1, Ordering::Relaxed);
        tracing::info!(c, "reload signaled");
    }

    pub async fn install_signal_handler(&self) {
        let mut hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
            .expect("failed to install SIGHUP handler");

        while hup.recv().await.is_some() {
            tracing::info!("SIGHUP received");
            self.notify_reload();
        }
    }
}
