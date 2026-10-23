// SPDX-License-Identifier: GPL-3.0-or-later
use crate::sync_cancellation::SyncOperation;
use crate::{CoreError, Engine, NextcloudSettings, SyncCancellation, SyncFailureKind};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NextcloudFolderMigrationReport {
    pub source_folder: String,
    pub target_folder: String,
    pub copied_files: u32,
    pub target_files: Vec<String>,
    pub verified: bool,
}
struct Lease<'a>(&'a AtomicBool);
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
#[cfg_attr(feature = "ffi", uniffi::export)]
impl Engine {
    /// Opt-in checked copy. Save the returned destination in native preferences only
    /// after verified success. The source is retained and local pending edits untouched.
    pub fn migrate_nextcloud_folder(
        &self,
        settings: NextcloudSettings,
        target_folder: String,
        cancellation: Arc<SyncCancellation>,
        timeout_ms: u64,
    ) -> Result<NextcloudFolderMigrationReport, CoreError> {
        if !cancellation.begin() {
            return Err(CoreError::Transient {
                kind: SyncFailureKind::Cancelled,
                message: "Folder copy was cancelled or already used".into(),
            });
        }
        let _operation = SyncOperation(&cancellation);
        let cfg = crate::momentum_sync::NextcloudCfg {
            server_url: settings.server_url,
            user_name: settings.user_name,
            password: settings.password,
            folder: settings.folder.clone(),
            compress: settings.compress,
            encrypt_key: settings.encryption_password.filter(|k| !k.is_empty()),
        };
        if !cfg.is_complete() {
            return Err(CoreError::NotConfigured);
        }
        #[cfg(feature = "p2p")]
        let nearby = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        #[cfg(feature = "p2p")]
        if nearby.is_some() {
            return Err(CoreError::Busy);
        }
        let generation = {
            let g = self.lock();
            if self.syncing.swap(true, Ordering::SeqCst) {
                return Err(CoreError::Busy);
            }
            g.sync_generation
        };
        let _lease = Lease(&self.syncing);
        #[cfg(feature = "p2p")]
        drop(nearby);
        let dir = self.lock().store.dir().to_path_buf();
        let upgraded =
            crate::momentum_sync::optimized::require_optimized(&cfg, &dir).map_err(|e| CoreError::Transient {
                kind: e.kind().into(),
                message: e.to_string(),
            })?;
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let start = std::time::Instant::now();
        let current = || !cancellation.is_cancelled() && self.lock().sync_generation == generation;
        if upgraded && !settings.optimized {
            return Err(CoreError::Transient {
                kind: SyncFailureKind::Cancelled,
                message: "Optimized sync is required for this connection".into(),
            });
        }
        let r = if settings.optimized {
            let mut destination = cfg.clone();
            destination.folder = target_folder.trim_matches('/').into();
            // Pin before publication, including a failed or uncertain destination copy.
            crate::momentum_sync::optimized::pin_optimized(&destination, &dir)
                .and_then(|_| {
                    let report = crate::momentum_sync::optimized::copy_collection_guarded(
                        &cfg,
                        &target_folder,
                        timeout,
                        &current,
                    )?;
                    crate::momentum_sync::optimized::mark_initialized(&destination, &dir)?;
                    Ok(report)
                })
                .map_err(|e| CoreError::Transient {
                    kind: e.kind().into(),
                    message: e.to_string(),
                })?
        } else {
            crate::momentum_sync::migrate_folder_guarded(
                &cfg,
                &target_folder,
                timeout.saturating_sub(start.elapsed()),
                current,
            )
            .map_err(|e| CoreError::Transient {
                kind: e.kind().into(),
                message: e.to_string(),
            })?
        };
        let g = self.lock();
        if g.sync_generation != generation || !cancellation.begin_commit() {
            return Err(CoreError::Transient {
                kind: SyncFailureKind::Cancelled,
                message: "Folder copy was cancelled or superseded".into(),
            });
        }
        Ok(NextcloudFolderMigrationReport {
            source_folder: settings.folder,
            target_folder: target_folder.trim_matches('/').into(),
            copied_files: r.copied_files,
            target_files: r.target_files,
            verified: true,
        })
    }
}
