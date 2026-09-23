// SPDX-License-Identifier: GPL-3.0-or-later
//! Undo journal for the existing three-file store. No backup or sync schema changes.
//! One process owner must serialize writes. A published journal is complete and
//! durable before any live file changes; recovery is idempotent after interruption.
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(super) const FILES: [&str; 3] = ["state.json", "pending.json", "meta.json"];
const JOURNAL: &str = ".momentum-transaction";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    present: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Prepared,
    Replaced(usize),
    Durable,
    Committed,
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}
fn regular_file(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "store entry is not a regular file",
        )),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}
fn write_synced(path: &Path, data: &[u8]) -> io::Result<()> {
    regular_file(path)?;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(data)?;
    file.sync_all()
}
fn private_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}
fn exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}
fn journal_manifest(journal: &Path) -> io::Result<Manifest> {
    if !fs::symlink_metadata(journal)?.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid store recovery directory",
        ));
    }
    if !regular_file(&journal.join("manifest.json"))? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing store recovery manifest",
        ));
    }
    let manifest: Manifest = serde_json::from_slice(&fs::read(journal.join("manifest.json"))?)?;
    let mut names = std::collections::HashSet::new();
    if manifest.version != 1
        || manifest
            .present
            .iter()
            .any(|name| !FILES.contains(&name.as_str()) || !names.insert(name))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported store recovery manifest",
        ));
    }
    // Validate the entire image before changing any live path.
    for name in &manifest.present {
        if !regular_file(&journal.join(name))? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete store recovery image",
            ));
        }
    }
    Ok(manifest)
}
fn retire(dir: &Path, journal: &Path) -> io::Result<PathBuf> {
    let retired = dir.join(format!(".momentum-transaction.complete-{}", sp_model::new_id()));
    fs::rename(journal, &retired)?;
    if let Err(error) = sync_dir(dir) {
        // Leave the undo record discoverable if publication could not be confirmed.
        fs::rename(&retired, journal)
            .map_err(|_| io::Error::other("store commit is uncertain; recovery could not be restored"))?;
        return Err(error);
    }
    Ok(retired)
}
fn recover_from(dir: &Path, journal: &Path) -> io::Result<()> {
    let manifest = journal_manifest(journal)?;
    for name in FILES {
        let destination = dir.join(name);
        if manifest.present.iter().any(|n| n == name) {
            let tmp = dir.join(format!("{name}.recovery.tmp"));
            regular_file(&tmp)?;
            fs::copy(journal.join(name), &tmp)?;
            File::open(&tmp)?.sync_all()?;
            fs::rename(tmp, destination)?;
        } else if regular_file(&destination)? {
            fs::remove_file(destination)?;
        }
    }
    sync_dir(dir)?;
    let retired = retire(dir, journal)?;
    // All live files are durable now. Cleanup failure cannot undo the commit.
    let _ = fs::remove_dir_all(retired);
    Ok(())
}
pub(super) fn recover(dir: &Path) -> io::Result<()> {
    let journal = dir.join(JOURNAL);
    if exists(&journal)? {
        recover_from(dir, &journal)?;
    }
    Ok(())
}
/// Only discard unpublished or durably retired records, after recovery succeeds.
/// Startup owns the directory; active writes must not run concurrently with load.
pub(super) fn cleanup(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let suffix = name
            .strip_prefix(".momentum-transaction.prepare-")
            .or_else(|| name.strip_prefix(".momentum-transaction.complete-"));
        let Some(suffix) = suffix else { continue };
        let canonical_uuid = suffix.len() == 36
            && suffix.bytes().enumerate().all(|(i, byte)| {
                if [8, 13, 18, 23].contains(&i) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            });
        if canonical_uuid && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}
fn prepare(dir: &Path) -> io::Result<PathBuf> {
    let scratch = dir.join(format!(".momentum-transaction.prepare-{}", sp_model::new_id()));
    private_dir(&scratch)?;
    let scratch = Scratch(scratch);
    let mut present = vec![];
    for name in FILES {
        if regular_file(&dir.join(name))? {
            fs::copy(dir.join(name), scratch.0.join(name))?;
            File::open(scratch.0.join(name))?.sync_all()?;
            present.push(name.to_string());
        }
    }
    write_synced(
        &scratch.0.join("manifest.json"),
        &serde_json::to_vec(&Manifest { version: 1, present })?,
    )?;
    sync_dir(&scratch.0)?;
    let journal = dir.join(JOURNAL);
    fs::rename(&scratch.0, &journal)?;
    sync_dir(dir)?;
    Ok(journal)
}
pub(super) fn save(dir: &Path, images: [Vec<u8>; 3]) -> io::Result<()> {
    save_with_hook(dir, images, |_| {})
}
fn save_with_hook(dir: &Path, images: [Vec<u8>; 3], checkpoint: impl Fn(Step)) -> io::Result<()> {
    recover(dir)?;
    let journal = prepare(dir)?;
    let result = (|| {
        checkpoint(Step::Prepared);
        // Prepare every new file before replacing any member of the live image.
        for (name, data) in FILES.iter().zip(&images) {
            write_synced(&dir.join(format!("{name}.tmp")), data)?;
        }
        for (index, name) in FILES.iter().enumerate() {
            fs::rename(dir.join(format!("{name}.tmp")), dir.join(name))?;
            checkpoint(Step::Replaced(index));
        }
        sync_dir(dir)?;
        checkpoint(Step::Durable);
        let retired = retire(dir, &journal)?;
        checkpoint(Step::Committed);
        let _ = fs::remove_dir_all(retired);
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            recover(dir).map_err(|_| {
                io::Error::other("store write failed and recovery is pending; do not overwrite the store")
            })?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests;
