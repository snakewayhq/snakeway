use arc_swap::ArcSwapOption;
use snakeway_engine::runtime::diff::RejectReason;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::watch;

static RELOAD_EPOCH: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct ReloadEvent {
    pub epoch: u64,
}

/// The result of one reload that the control plane finished.
#[derive(Clone, Debug, PartialEq)]
pub enum ReloadOutcome {
    Applied,
    UpgradeStarted,
    UpgradeFailed {
        error: String,
    },
    Rejected {
        reason: RejectReason,
        settings: Vec<&'static str>,
    },
    LoadFailed {
        error: String,
    },
    BuildFailed {
        error: String,
    },
}

/// The last reload that finished, with the epoch of the reload request that it answered.
#[derive(Clone, Debug, PartialEq)]
pub struct ReloadStatus {
    pub epoch: u64,
    pub outcome: ReloadOutcome,
}

impl ReloadStatus {
    pub fn to_json(&self) -> serde_json::Value {
        let epoch = self.epoch;
        match &self.outcome {
            ReloadOutcome::Applied => serde_json::json!({ "epoch": epoch, "result": "applied" }),
            ReloadOutcome::UpgradeStarted => {
                serde_json::json!({ "epoch": epoch, "result": "upgrade_started" })
            }
            ReloadOutcome::UpgradeFailed { error } => {
                serde_json::json!({ "epoch": epoch, "result": "upgrade_failed", "error": error })
            }
            ReloadOutcome::Rejected { reason, settings } => serde_json::json!({
                "epoch": epoch,
                "result": "rejected",
                "reason": reason.as_str(),
                "settings": settings,
            }),
            ReloadOutcome::LoadFailed { error } => {
                serde_json::json!({ "epoch": epoch, "result": "load_failed", "error": error })
            }
            ReloadOutcome::BuildFailed { error } => {
                serde_json::json!({ "epoch": epoch, "result": "build_failed", "error": error })
            }
        }
    }
}

#[derive(Clone)]
pub struct ReloadHandle {
    tx: watch::Sender<ReloadEvent>,
    last: Arc<ArcSwapOption<ReloadStatus>>,
}

impl Default for ReloadHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl ReloadHandle {
    pub fn new() -> Self {
        let (tx, _) = watch::channel(ReloadEvent { epoch: 0 });
        Self {
            tx,
            last: Arc::new(ArcSwapOption::from(None)),
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<ReloadEvent> {
        self.tx.subscribe()
    }

    pub fn notify_reload(&self) -> u64 {
        let epoch = RELOAD_EPOCH.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.tx.send(ReloadEvent { epoch });
        tracing::info!(epoch, "reload signaled");
        epoch
    }

    /// Record the result of the reload that answered the request with this epoch.
    pub fn record_outcome(&self, epoch: u64, outcome: ReloadOutcome) {
        self.last
            .store(Some(Arc::new(ReloadStatus { epoch, outcome })));
    }

    /// The last reload that finished, or `None` before the first one.
    pub fn last_status(&self) -> Option<Arc<ReloadStatus>> {
        self.last.load_full()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_status_is_empty_before_any_outcome() {
        // Arrange
        let handle = ReloadHandle::new();

        // Act
        let status = handle.last_status();

        // Assert
        assert!(status.is_none());
    }

    #[test]
    fn recorded_outcome_is_visible_through_a_clone() {
        // Arrange
        let handle = ReloadHandle::new();
        let clone = handle.clone();

        // Act
        handle.record_outcome(7, ReloadOutcome::Applied);

        // Assert
        let status = clone.last_status().expect("the outcome must be recorded");
        assert_eq!(
            *status,
            ReloadStatus {
                epoch: 7,
                outcome: ReloadOutcome::Applied
            }
        );
    }

    #[test]
    fn rejected_status_serializes_reason_and_settings() {
        // Arrange
        let status = ReloadStatus {
            epoch: 3,
            outcome: ReloadOutcome::Rejected {
                reason: RejectReason::RestartRequired,
                settings: vec!["server.pid_file", "server.upgrade.sock"],
            },
        };

        // Act
        let json = status.to_json();

        // Assert
        assert_eq!(
            json,
            serde_json::json!({
                "epoch": 3,
                "result": "rejected",
                "reason": "restart_required",
                "settings": ["server.pid_file", "server.upgrade.sock"],
            })
        );
    }

    #[test]
    fn load_failed_status_serializes_error() {
        // Arrange
        let status = ReloadStatus {
            epoch: 4,
            outcome: ReloadOutcome::LoadFailed {
                error: "unknown field".to_string(),
            },
        };

        // Act
        let json = status.to_json();

        // Assert
        assert_eq!(
            json,
            serde_json::json!({ "epoch": 4, "result": "load_failed", "error": "unknown field" })
        );
    }
}
