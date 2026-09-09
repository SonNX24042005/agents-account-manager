use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Write a file through a same-directory temporary file and atomically replace it on Unix.
/// The temporary file is created with the requested mode, so secrets are never briefly public.
pub fn atomic_write(path: &Path, contents: &[u8], unix_mode: u32) -> Result<()> {
    #[cfg(not(unix))]
    let _ = unix_mode;

    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory {}", parent.display()))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("secure-file");
    let temp_path = parent.join(format!(".{}.{}.tmp", file_name, uuid::Uuid::new_v4()));

    let result = (|| -> Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(unix_mode);
        }

        let mut file = options
            .open(&temp_path)
            .with_context(|| format!("Failed to create {}", temp_path.display()))?;
        file.write_all(contents)
            .with_context(|| format!("Failed to write {}", temp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("Failed to sync {}", temp_path.display()))?;
        drop(file);

        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("Failed to replace {}", path.display()))?;
        }

        fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to replace {}", path.display()))?;
        #[cfg(unix)]
        {
            fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .with_context(|| format!("Failed to sync directory {}", parent.display()))?;
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

/// Preserve the current file before changing user-managed configuration.
pub fn backup(path: &Path) -> Result<Option<std::path::PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Failed to inspect {}", path.display()))?;
    anyhow::ensure!(
        metadata.file_type().is_file(),
        "Refusing to back up a non-regular file"
    );

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid backup file name: {}", path.display()))?;
    let backup_path = path.with_file_name(format!("{}.aam.bak", file_name));
    let contents = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mode = existing_mode_or(path, 0o600);
    atomic_write(&backup_path, &contents, mode)?;
    Ok(Some(backup_path))
}

pub fn existing_mode_or(path: &Path, fallback: u32) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|metadata| metadata.permissions().mode() & 0o777)
            .unwrap_or(fallback)
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        fallback
    }
}
