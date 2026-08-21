use std::path::PathBuf;
use std::io::Error;

pub const DATA_DIR_KEY: &str = "ATLAS_DATA_DIR";
pub const SYNC_URL_KEY: &str = "ATLAS_SYNC_URL";

/// Resolves the on-disk data directory for a namespaced local database, creating it if
/// missing. Honors `ATLAS_DATA_DIR` first, then `~/.atlas/<namespace>/data`, then `fallback`.
pub fn get_data_dir(namespace: &str, fallback: &str) -> Result<PathBuf, Error> {
    let data_dir = std::env::var(DATA_DIR_KEY)
        .map(|path| PathBuf::from(&path))
        .unwrap_or_else(|error| match dirs::home_dir() {
            Some(home_dir) => home_dir.join(".atlas").join(namespace).join("data"),
            None => {
                tracing::info!("Couldn't get user home directory: {:}; using fallback", error);
                tracing::debug!("Hint: Set the {} environment variable to override the local database path.", DATA_DIR_KEY);
                PathBuf::from(fallback)
            }
        });

    if !std::path::Path::new(&data_dir).exists() {
        std::fs::create_dir_all(&data_dir)?;
    }

    Ok(data_dir)
}
