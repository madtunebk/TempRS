// Core streaming facades. These currently delegate to existing utils
// and allow incremental migration without changing call sites.

use futures_util::StreamExt;
use minimp3::{Decoder as Mp3Decoder, Frame};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc};

/// Stream from actual CDN URL with optional byte offset.
/// Sends decoded i16 frames to `sample_tx` and optionally to `fft_download_tx`.
pub async fn stream_from_cdn(
    actual_url: &str,
    byte_offset: Option<u64>,
    sample_tx: Sender<Vec<i16>>,
    fft_download_tx: Option<Sender<Vec<i16>>>,
    shutdown: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = crate::utils::http::streaming_client();
    let mut req = client.get(actual_url);
    if let Some(off) = byte_offset {
        req = req.header("Range", format!("bytes={}-", off));
    }
    let resp = req.send().await?;
    if !(resp.status().is_success() || resp.status().as_u16() == 206) {
        return Err(format!("CDN status {}", resp.status()).into());
    }

    let mut buffer: Vec<u8> = Vec::new();
    let mut buffer_frames_sent: usize = 0;
    // Keep logic minimal here; idle handling can be layered by caller if needed
    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        if shutdown.load(Ordering::Relaxed) {
            finished.store(true, Ordering::Relaxed);
            return Ok(());
        }
        match item {
            Ok(chunk) => {
                buffer.extend_from_slice(&chunk);
                // bytes received; continue

                let mut decoder = Mp3Decoder::new(&buffer[..]);
                let mut frame_index = 0;
                while let Ok(Frame { data, .. }) = decoder.next_frame() {
                    if frame_index >= buffer_frames_sent {
                        if sample_tx.send(data.clone()).is_err() {
                            finished.store(true, Ordering::Relaxed);
                            return Ok(());
                        }
                        if let Some(tx) = &fft_download_tx {
                            let _ = tx.send(data.clone());
                        }
                        buffer_frames_sent = frame_index + 1;
                    }
                    frame_index += 1;
                }

                if buffer.len() > 5 * 1024 * 1024 {
                    let keep_size = 2 * 1024 * 1024;
                    let trim = buffer.len() - keep_size;
                    buffer.drain(0..trim);
                    buffer_frames_sent = 0;
                }
            }
            Err(e) => {
                log::warn!("[Streaming] stream error: {}", e);
                break;
            }
        }

        // Idle handling left to caller (mediaplay) to decide policy; no early finish here
        // optional backoff could be applied by the caller; do nothing here
    }

    let mut decoder = Mp3Decoder::new(&buffer[..]);
    let mut frame_index = 0;
    while let Ok(Frame { data, .. }) = decoder.next_frame() {
        if frame_index >= buffer_frames_sent {
            let _ = sample_tx.send(data.clone());
            if let Some(tx) = &fft_download_tx {
                let _ = tx.send(data.clone());
            }
        }
        frame_index += 1;
    }

    finished.store(true, Ordering::Relaxed);
    Ok(())
}
