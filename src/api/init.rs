#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::utils::file_wasm;
use crate::vfs::wasm_helpers::{get_vfs, vfs_error_to_js};
use crate::vfs::{VercelKvVfs, VfsError, VirtualFileSystem};
use log;
use serde_json::{json, Value};

// --- Wasm Bindgen Exports ---

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init_wasm_logging(level: Option<String>) -> Result<(), JsValue> {
    use log::Level;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Static flag to track if logger has been initialized
    static LOGGER_INITIALIZED: AtomicBool = AtomicBool::new(false);

    // Set up the panic hook for better stack traces
    crate::utils::set_panic_hook();

    // Check if logger is already initialized
    if LOGGER_INITIALIZED.load(Ordering::SeqCst) {
        log::info!("Logger already initialized, skipping initialization");
        return Ok(());
    }

    let log_level = match level.as_deref() {
        Some("error") => Level::Error,
        Some("warn") => Level::Warn,
        Some("info") => Level::Info,
        Some("debug") => Level::Debug,
        Some("trace") => Level::Trace,
        _ => Level::Info, // Default to Info level
    };

    match console_log::init_with_level(log_level) {
        Ok(_) => {
            // Mark logger as initialized
            LOGGER_INITIALIZED.store(true, Ordering::SeqCst);
            log::info!(
                "WASM logging initialized at level: {} with stack trace support",
                log_level
            );
            Ok(())
        }
        Err(e) => {
            // If error is about logger already being set, consider it a success
            if e.to_string().contains("already initialized") {
                LOGGER_INITIALIZED.store(true, Ordering::SeqCst);
                log::debug!("Logger was already initialized by another part of the application");
                Ok(())
            } else {
                Err(JsValue::from_str(&format!(
                    "Failed to initialize logger: {}",
                    e
                )))
            }
        }
    }
}

/// Initializes the VFS, potentially loading data from GitHub if it's the first time.
/// Returns `true` if the GitHub load was triggered, `false` otherwise.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn initialize_subconverter_webapp() -> Result<bool, JsValue> {
    log::info!("initialize_subconverter_webapp called");
    let vfs = get_vfs().await.map_err(vfs_error_to_js)?;
    vfs.initialize_github_load().await.map_err(vfs_error_to_js)
}
