use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::watch;

static RELOAD_EPOCH: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct ReloadEvent {
    pub epoch: u64,
}

#[derive(Clone)]
pub struct ReloadHandle {
    tx: watch::Sender<ReloadEvent>,
}

impl ReloadHandle {
    pub fn new() -> Self {
        let (tx, _) = watch::channel(ReloadEvent { epoch: 0 });
        Self { tx }
    }

    pub fn subscribe(&self) -> watch::Receiver<ReloadEvent> {
        self.tx.subscribe()
    }

    pub fn notify_reload(&self) {
        let epoch = RELOAD_EPOCH.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.tx.send(ReloadEvent { epoch });
        tracing::info!(epoch, "reload signaled");
    }

    pub async fn install_signal_handler(&self) -> anyhow::Result<()> {
        let mut hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;

        while hup.recv().await.is_some() {
            tracing::info!("SIGHUP received");
            self.notify_reload();
        }
        Ok(())
    }
}
