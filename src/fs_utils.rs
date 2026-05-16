use anyhow::{Context, Result};
use std::path::Path;
use tracing::warn;

/// Move a file to the destination path.
///
/// Falls back to copy+remove when source and destination are on different filesystems.
pub fn move_file_with_fallback(source: &Path, destination: &Path) -> Result<()> {
    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(err) if err.raw_os_error() == Some(18) => {
            warn!(
                "Cross-device move detected (EXDEV), falling back to copy+remove: {} -> {}",
                source.display(),
                destination.display()
            );

            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create destination directory during fallback move: {}",
                        parent.display()
                    )
                })?;
            }

            std::fs::copy(source, destination).with_context(|| {
                format!(
                    "Failed to copy file during cross-device fallback move: {} -> {}",
                    source.display(),
                    destination.display()
                )
            })?;

            std::fs::remove_file(source).with_context(|| {
                format!(
                    "Failed to remove source file after fallback copy: {}",
                    source.display()
                )
            })?;

            Ok(())
        }
        Err(err) => Err(err).with_context(|| {
            format!(
                "Failed to move file to destination: {} -> {}",
                source.display(),
                destination.display()
            )
        }),
    }
}