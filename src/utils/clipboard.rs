/// Clipboard utilities for sharing track URLs and other content
use log::{error, info, warn};

/// Copy text to system clipboard
/// Returns true if successful, false otherwise
pub fn copy_to_clipboard(text: &str, context: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use arboard::Clipboard;
        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(text) {
                Ok(_) => {
                    info!("[Clipboard] Copied {} to clipboard: {}", context, text);
                    true
                }
                Err(e) => {
                    error!("[Clipboard] Failed to copy {} to clipboard: {}", context, e);
                    false
                }
            },
            Err(e) => {
                error!("[Clipboard] Failed to access clipboard: {}", e);
                false
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        warn!("[Clipboard] Clipboard not supported on WASM target");
        false
    }
}

/// Share a track URL by copying to clipboard
/// Returns true if successful, false otherwise
pub fn share_track_url(permalink_url: Option<&str>) -> bool {
    if let Some(url) = permalink_url {
        copy_to_clipboard(url, "track URL")
    } else {
        warn!("[Clipboard] No track URL available to share");
        false
    }
}
