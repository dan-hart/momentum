// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use crate::portable::{apply_preferences, Document};
use crate::{CoreError, Engine, Message, Outcome};
use momentum_model::{new_id, now_ms, AppData};
use momentum_oplog::Action;
use momentum_store::{Pending, Store};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct Recovery<T> {
    data: T,
    pending: Vec<Action>,
}

fn invalid_backup(_: serde_json::Error) -> CoreError {
    CoreError::Invalid {
        message: "Invalid backup or recovery data".into(),
    }
}

fn backup_notice(message: Message, changed: bool) -> Outcome {
    Outcome {
        message: Some(message),
        changed,
        ..Outcome::none()
    }
}

#[cfg_attr(feature = "ffi", uniffi::export)]
impl Engine {
    /// Replace local tasks from a backup after journaling a private, verified copy.
    pub fn import_backup(&self, path: String) -> Result<Outcome, CoreError> {
        self.import_backup_with_settings(path, false)
    }

    pub fn import_backup_with_settings(&self, path: String, restore_settings: bool) -> Result<Outcome, CoreError> {
        let backup: Value = serde_json::from_slice(&fs::read(path)?).map_err(invalid_backup)?;
        let settings = if restore_settings {
            backup
                .get("momentumSettings")
                .map(|v| Document::parse(&v.to_string()))
                .transpose()?
        } else {
            None
        };
        let data = AppData::from_backup(backup).map_err(invalid_backup)?;
        self.replace_from_backup(move |_| Ok((data, Vec::new())), settings)?;
        Ok(backup_notice(Message::BackupImported, true))
    }

    pub fn has_backup_recovery(&self) -> bool {
        self.lock().store.has_backup_recovery()
    }

    /// Restore tasks and requeue unsynced actions from the latest private recovery copy.
    pub fn restore_backup_recovery(&self) -> Result<Outcome, CoreError> {
        self.replace_from_backup(
            |store| {
                let bytes = store.read_backup_recovery()?.ok_or_else(|| CoreError::Invalid {
                    message: "No backup recovery copy is available".into(),
                })?;
                let image: Recovery<Value> = serde_json::from_slice(&bytes).map_err(invalid_backup)?;
                let data = AppData::from_snapshot(image.data).map_err(invalid_backup)?;
                Ok((data, image.pending))
            },
            None,
        )?;
        Ok(backup_notice(Message::BackupRecoveryRestored, true))
    }

    pub fn export_backup(&self, path: String) -> Result<Outcome, CoreError> {
        self.export_backup_with_settings(path, false)
    }

    pub fn export_backup_with_settings(&self, path: String, include_settings: bool) -> Result<Outcome, CoreError> {
        let path = Path::new(&path);
        let body = {
            let g = self.lock();
            if g.store.protects_backup_destination(path)? {
                return Err(CoreError::Invalid {
                    message: "Choose a destination outside the live task store".into(),
                });
            }
            let mut data = json!({"data": g.store.state, "timestamp": now_ms(), "crossModelVersion": 4.5});
            if include_settings {
                data["momentumSettings"] = json!(g.portable.payload.document());
            }
            serde_json::to_vec_pretty(&data).map_err(invalid_backup)?
        };
        let verification = write_backup(path, &body)?;
        Ok(backup_notice(verification, false))
    }
}

impl Engine {
    fn replace_from_backup(
        &self,
        load: impl FnOnce(&Store) -> Result<(AppData, Vec<Action>), CoreError>,
        settings: Option<Document>,
    ) -> Result<(), CoreError> {
        // Hold provider admission through the replacement journal commit. A stopped
        // runtime may have queued events, so the new epoch rejects those events.
        #[cfg(feature = "p2p")]
        let mut nearby = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        #[cfg(feature = "p2p")]
        self.stop_p2p_locked(&mut nearby);
        let mut g = self.lock();
        let (data, pending) = load(&g.store)?;
        let recovery = serde_json::to_vec(&Recovery {
            data: &g.store.state,
            pending: g.store.pending.iter().map(|pending| pending.action.clone()).collect(),
        })
        .map_err(invalid_backup)?;

        let mut store = g.store.clone();
        store.state = data;
        // A provider can publish our clock while the imported image is live. Queue
        // the original edits with new operations that follow the current clock.
        store.pending = pending
            .into_iter()
            .map(|action| {
                let op = action.to_op(&store.meta.client_id, &mut store.meta.vector_clock);
                Pending { op, action }
            })
            .collect();
        store.meta.p2p_restore_epoch = store.meta.p2p_restore_epoch.wrapping_add(1);
        store.meta.p2p_bootstrapped = true;
        store.meta.p2p_applied_ops.clear();
        let portable = settings.map(|document| {
            let mut portable = g.portable.clone();
            portable.stage(document, &store.meta.client_id);
            if portable.payload.document() != g.portable.payload.document() {
                portable.revision = g.portable.revision.saturating_add(1);
            }
            portable
        });
        let portable_bytes = portable.as_ref().map(|p| p.bytes()).transpose()?;
        store.save_with_backup_recovery(&recovery, portable_bytes.as_deref())?;
        g.sync_generation = g.sync_generation.wrapping_add(1);
        self.clear_nextcloud_caches();
        g.store = store;
        if let Some(portable) = portable {
            g.portable = portable;
            apply_preferences(&mut g);
        }
        g.undo.clear();
        g.invalidate();
        Ok(())
    }
}

struct BackupScratch(PathBuf);

impl Drop for BackupScratch {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn write_backup(path: &Path, expected: &[u8]) -> Result<Message, CoreError> {
    write_backup_with(path, expected, File::write_all)
}

fn write_backup_with(
    path: &Path,
    expected: &[u8],
    write: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
) -> Result<Message, CoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(io::Error::new(ErrorKind::InvalidInput, "backup destination must be a regular file").into());
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let staged = parent.join(format!(".momentum-backup-{}.tmp", new_id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&staged)?;
    let staged = BackupScratch(staged);
    write(&mut file, expected)?;
    file.sync_all()?;
    drop(file);

    if fs::read(&staged.0)? != expected {
        return Err(io::Error::other("staged backup did not match the exported data").into());
    }
    // Keep a previously exported file intact on write or verification failure.
    fs::rename(&staged.0, path)?;
    match File::open(parent).and_then(|file| file.sync_all()) {
        Err(error) if !matches!(error.kind(), ErrorKind::Unsupported | ErrorKind::PermissionDenied) => {
            return Err(error.into());
        }
        _ => {}
    }
    match fs::read(path) {
        Ok(actual) if actual == expected => Ok(Message::BackupExported),
        Ok(_) => Err(io::Error::other("saved backup did not match the exported data").into()),
        Err(error) if matches!(error.kind(), ErrorKind::PermissionDenied | ErrorKind::Unsupported) => {
            Ok(Message::BackupExportVerificationUnavailable)
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn export_rejects_live_store_paths_and_symlinks_without_mutating_them() {
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::open_checked(directory.path().to_string_lossy().into_owned()).unwrap();
        engine.add_task_for_today("Preserve me".into());
        let live = directory.path().join("state.json");
        let alias = directory.path().join("alias.json");
        std::os::unix::fs::symlink(&live, &alias).unwrap();
        let before = fs::read(&live).unwrap();

        for path in [&live, &alias] {
            assert!(engine.export_backup(path.to_string_lossy().into_owned()).is_err());
        }
        assert_eq!(fs::read(&live).unwrap(), before);
        assert!(alias.is_symlink());
    }

    #[test]
    fn interrupted_export_write_preserves_previous_destination_and_removes_scratch() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("backup.json");
        fs::write(&path, b"previous backup").unwrap();

        let result = write_backup_with(&path, b"new backup", |file, _| {
            file.write_all(b"partial")?;
            Err(io::Error::other("fixture write failure"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"previous backup");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
