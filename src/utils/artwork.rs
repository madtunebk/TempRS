use image::ImageError;
use std::sync::mpsc::{channel, Sender};

// ====================================
// COLOR EXTRACTION (Ambient Effects)
// ====================================

/// Extract dominant color from an image (RGB)
pub fn extract_dominant_color(img: &egui::ColorImage) -> egui::Color32 {
    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count: u64 = 0;

    // Sample every 4th pixel for performance (still gives good results)
    for (i, pixel) in img.pixels.iter().enumerate() {
        if i % 4 == 0 {
            r_sum += pixel[0] as u64;
            g_sum += pixel[1] as u64;
            b_sum += pixel[2] as u64;
            count += 1;
        }
    }

    if count == 0 {
        return egui::Color32::from_rgb(255, 85, 0); // Fallback to app orange
    }

    let r = (r_sum / count) as u8;
    let g = (g_sum / count) as u8;
    let b = (b_sum / count) as u8;

    // Boost saturation by 30% for more vibrant glow
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    if delta == 0 {
        return egui::Color32::from_rgb(r, g, b);
    }

    // Apply saturation boost
    let boost = 1.3; // COLOR VIBRANCY: Change to 1.0-2.0 (1.0=original, 1.3=30% more vibrant, 1.8=80% punchier)
    let r_boosted = ((r as f32 - min as f32) * boost + min as f32).min(255.0) as u8;
    let g_boosted = ((g as f32 - min as f32) * boost + min as f32).min(255.0) as u8;
    let b_boosted = ((b as f32 - min as f32) * boost + min as f32).min(255.0) as u8;

    egui::Color32::from_rgb(r_boosted, g_boosted, b_boosted)
}

/// Extract ambient edge colors (Ambilight/YouTube style)
/// Returns [top, right, bottom, left] colors sampled from image edges
pub fn extract_edge_colors(img: &egui::ColorImage) -> [egui::Color32; 4] {
    let width = img.width();
    let height = img.height();

    if width == 0 || height == 0 {
        let fallback = egui::Color32::from_rgb(255, 85, 0);
        return [fallback, fallback, fallback, fallback];
    }

    // EDGE THICKNESS (10% of size, min 6px)
    let edge_h = (height / 10).max(6);
    let edge_w = (width / 10).max(6);

    // ✔ TOP EDGE  (full width, small vertical slice)
    let top = sample_edge_region(img, 0, 0, width, edge_h);

    // ✔ BOTTOM EDGE (full width, bottom slice)
    let bottom = sample_edge_region(img, 0, height - edge_h, width, height);

    // ✔ LEFT EDGE (small width slice, full height)
    let left = sample_edge_region(img, 0, 0, edge_w, height);

    // ✔ RIGHT EDGE (small width slice, full height)
    let right = sample_edge_region(img, width - edge_w, 0, width, height);

    // ✔ Apply vibrancy
    let boost = 1.35;
    [
        boost_saturation(top, boost),    // TOP
        boost_saturation(right, boost),  // RIGHT
        boost_saturation(bottom, boost), // BOTTOM
        boost_saturation(left, boost),   // LEFT
    ]
}

/// Sample average color from a rectangular region
fn sample_edge_region(
    img: &egui::ColorImage,
    x_start: usize,
    y_start: usize,
    x_end: usize,
    y_end: usize,
) -> egui::Color32 {
    let w = img.width();
    let h = img.height();

    let xs = x_start.min(w);
    let xe = x_end.min(w);
    let ys = y_start.min(h);
    let ye = y_end.min(h);

    let mut r_sum = 0u64;
    let mut g_sum = 0u64;
    let mut b_sum = 0u64;
    let mut count = 0u64;

    for y in ys..ye {
        for x in xs..xe {
            let idx = y * w + x;
            if let Some(px) = img.pixels.get(idx) {
                r_sum += px[0] as u64;
                g_sum += px[1] as u64;
                b_sum += px[2] as u64;
                count += 1;
            }
        }
    }

    if count == 0 {
        return egui::Color32::from_rgb(255, 85, 0);
    }

    egui::Color32::from_rgb(
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
    )
}

/// Boost color saturation
fn boost_saturation(color: egui::Color32, boost: f32) -> egui::Color32 {
    let [r, g, b, a] = color.to_array();

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    if delta == 0 {
        return color;
    }

    let r_boosted = ((r as f32 - min as f32) * boost + min as f32).min(255.0) as u8;
    let g_boosted = ((g as f32 - min as f32) * boost + min as f32).min(255.0) as u8;
    let b_boosted = ((b as f32 - min as f32) * boost + min as f32).min(255.0) as u8;

    egui::Color32::from_rgba_premultiplied(r_boosted, g_boosted, b_boosted, a)
}

// ====================================
// CENTRALIZED ARTWORK LOADING
// ====================================

/// Load artwork texture for a track, using all available caching layers
/// Returns texture handle if available, None otherwise
///
/// Priority:
/// 1. Memory cache (by URL if track has artwork_url)
/// 2. Memory cache (by track ID)
/// 3. Disk cache (by track ID)
/// 4. no_artwork.png placeholder
/// 5. None (triggers gray box fallback in UI)
pub fn load_track_artwork(
    app: &mut crate::app::player_app::MusicPlayerApp,
    ctx: &egui::Context,
    track_id: u64,
    artwork_url: Option<&str>,
    cache_key: &str,
) -> Option<egui::TextureHandle> {
    // 1. Check memory cache by URL (if track has artwork_url)
    if let Some(url) = artwork_url {
        let url_high = url.replace("-large.jpg", "-t500x500.jpg");
        if let Some(texture) = app.ui.thumb_cache.get(&url_high) {
            log::debug!(
                "[Artwork] Track {} found in memory cache (by URL)",
                track_id
            );
            return Some(texture.clone());
        }
    }

    // 2. Check memory cache by track ID
    if let Some(texture) = app.ui.thumb_cache.get(cache_key) {
        log::debug!("[Artwork] Track {} found in memory cache (by ID)", track_id);
        return Some(texture.clone());
    }

    // 3. Check disk cache by track ID
    if let Some(cached_data) = crate::utils::cache::load_artwork_cache(track_id) {
        log::debug!(
            "[Artwork] Track {} found in disk cache ({} bytes)",
            track_id,
            cached_data.len()
        );
        if let Ok(decoded) = image::load_from_memory(&cached_data) {
            let rgba = decoded.to_rgba8();
            let (w, h) = rgba.dimensions();
            let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let texture = ctx.load_texture(cache_key, img, egui::TextureOptions::LINEAR);

            // Cache in memory for next time
            app.ui
                .thumb_cache
                .insert(cache_key.to_string(), texture.clone());
            log::debug!(
                "[Artwork] Track {} loaded from disk cache ({}x{})",
                track_id,
                w,
                h
            );

            return Some(texture);
        } else {
            log::warn!(
                "[Artwork] Track {} disk cache corrupted, will re-download",
                track_id
            );
        }
    }

    // 4. No texture available - return None to trigger download in calling code
    log::debug!("[Artwork] Track {} not in cache, needs download", track_id);
    None
}

// ====================================
// BACKGROUND ARTWORK FETCHING
// ====================================

/// Fetch artwork from URL with high-res replacement and caching using track ID
pub fn fetch_artwork(
    track_id: u64,
    artwork_url: String,
) -> (Sender<()>, std::sync::mpsc::Receiver<egui::ColorImage>) {
    let (tx, rx) = channel::<egui::ColorImage>();
    let (cancel_tx, _cancel_rx) = channel::<()>();

    std::thread::spawn(move || {
        // Check cache first using track ID (synchronous check before async work)
        if let Some(cached_bytes) = crate::utils::cache::load_artwork_cache(track_id) {
            if let Ok(decoded) = image::load_from_memory(&cached_bytes) {
                let rgba = decoded.to_rgba8();
                let (w, h) = rgba.dimensions();
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                let _ = tx.send(img);
                return;
            }
        }

        // Not in cache, fetch from network using async helper
        let rt = match crate::utils::error_handling::create_runtime() {
            Ok(r) => r,
            Err(e) => {
                log::error!("[Artwork] {}", e);
                return;
            }
        };

        rt.block_on(async {
            // Replace image size suffix with -t500x500.jpg for higher quality
            let high_res_url = if artwork_url.contains("-large.jpg") {
                artwork_url.replace("-large.jpg", "-t500x500.jpg")
            } else if artwork_url.contains("-t500x500.jpg") {
                artwork_url.to_string()
            } else {
                artwork_url.replace(".jpg", "-t500x500.jpg")
            };

            let client = crate::utils::http::client();

            // Try up to 2 times with exponential backoff (reduced from 3 to save API calls)
            let mut success = false;
            for attempt in 1..=2 {
                match client.get(&high_res_url).send().await {
                    Ok(resp) => {
                        if let Ok(bytes) = resp.bytes().await {
                            // Save to cache using track ID
                            let _ =
                                crate::utils::cache::save_artwork_cache(track_id, &bytes, false);

                            if let Ok(decoded) = image::load_from_memory(&bytes) {
                                let rgba = decoded.to_rgba8();
                                let (w, h) = rgba.dimensions();
                                let img = egui::ColorImage::from_rgba_unmultiplied(
                                    [w as usize, h as usize],
                                    &rgba,
                                );
                                let _ = tx.send(img);
                                success = true;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("[Artwork] Attempt {}/2 failed: {}", attempt, e);
                        if attempt < 2 {
                            // Backoff: 500ms
                            tokio::time::sleep(tokio::time::Duration::from_millis(500))
                            .await;
                        }
                    }
                }
            }

            // If all attempts failed, cache and send no_artwork placeholder
            if !success {
                log::warn!("[Artwork] All 2 attempts failed, caching placeholder");
                send_placeholder(&tx, track_id);
            }
        });
    });

    (cancel_tx, rx)
}

/// Send the no_artwork.png placeholder
fn send_placeholder(tx: &Sender<egui::ColorImage>, track_id: u64) {
    let no_artwork_bytes = include_bytes!("../assets/no_artwork.png");

    // Cache the placeholder so we don't retry on next launch
    let _ = crate::utils::cache::save_artwork_cache(track_id, no_artwork_bytes, true);

    if let Ok(decoded) = image::load_from_memory(no_artwork_bytes) {
        let rgba = decoded.to_rgba8();
        let (w, h) = rgba.dimensions();
        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let _ = tx.send(img);
    }
}

/// Load artwork from memory (for immediate display)
pub fn load_artwork_from_bytes(bytes: &[u8]) -> Result<egui::ColorImage, ImageError> {
    let decoded = image::load_from_memory(bytes)?;
    let rgba = decoded.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        &rgba,
    ))
}

/// Load thumbnail artwork with cache-first pattern (sync check, async download)
/// Used by search and home screens for grid thumbnails
pub fn load_thumbnail_artwork(
    app: &mut crate::app::player_app::MusicPlayerApp,
    ctx: &egui::Context,
    track_id: u64,
    url: String,
    validate_before_cache: bool,
) {
    // Skip if already loaded in memory
    if app.ui.thumb_cache.contains_key(&url) {
        return;
    }

    // Skip download if already pending (prevents duplicate spawns during race condition)
    if app.ui.thumb_pending.contains_key(&url) {
        return;
    }

    // FAST PATH: Try disk cache using track ID (sync, check every frame until loaded)
    if let Some(cached_data) = crate::utils::cache::load_artwork_cache(track_id) {
        log::debug!(
            "[Artwork] Track {} loading from disk cache in background loader ({} bytes)",
            track_id,
            cached_data.len()
        );
        if let Ok(img) = image::load_from_memory(&cached_data) {
            let size = [img.width() as _, img.height() as _];
            let image_buffer = img.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            let texture = ctx.load_texture(&url, color_image, egui::TextureOptions::LINEAR);
            app.ui.thumb_cache.insert(url.clone(), texture);
            app.ui.thumb_pending.remove(&url);
            log::info!(
                "[Artwork] Track {} loaded from disk cache ({}x{})",
                track_id,
                size[0],
                size[1]
            );
            return;
        } else {
            log::warn!(
                "[Artwork] Track {} disk cache corrupted, will download fresh",
                track_id
            );
        }
    }

    // Mark as pending BEFORE starting download to prevent race condition
    app.ui.thumb_pending.insert(url.clone(), true);
    log::debug!(
        "[Artwork] Track {} marked as pending for download",
        track_id
    );

    // SLOW PATH: Not in cache → download async with retry
    log::info!(
        "[Artwork] Track {} starting download from {}",
        track_id,
        url
    );
    let ctx_clone = ctx.clone();
    let url_clone = url.clone();

    crate::utils::async_helper::spawn_fire_and_forget(move || {
        Box::pin(async move {
            let client = crate::utils::http::client();
            let mut success = false;

            // Try up to 2 times with exponential backoff (reduced from 3 to save API calls)
            for attempt in 1..=2 {
                match client.get(&url_clone).send().await {
                    Ok(resp) => {
                        if let Ok(bytes) = resp.bytes().await {
                            log::debug!(
                                "[Artwork] Track {} downloaded {} bytes (attempt {})",
                                track_id,
                                bytes.len(),
                                attempt
                            );
                            // Validate and save to cache using track ID
                            if let Ok(_img) = image::load_from_memory(&bytes) {
                                // Save to cache - next frame will load it via FAST PATH
                                let _ = crate::utils::cache::save_artwork_cache(
                                    track_id, &bytes, false,
                                );
                                log::info!(
                                    "[Artwork] Track {} download successful, saved to cache",
                                    track_id
                                );
                                // Request repaint to trigger FAST PATH on next frame
                                ctx_clone.request_repaint();
                                success = true;
                                break;
                            } else {
                                log::warn!(
                                    "[Artwork] Track {} invalid image data (attempt {}/2)",
                                    track_id,
                                    attempt
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "[Artwork] Track {} download failed (attempt {}/2): {}",
                            track_id,
                            attempt,
                            e
                        );
                        if attempt < 2 {
                            // Backoff: 500ms
                            tokio::time::sleep(tokio::time::Duration::from_millis(500))
                            .await;
                        }
                    }
                }
            }

            // If all attempts failed, save placeholder to prevent retry loops
            if !success && validate_before_cache {
                log::warn!(
                    "[Artwork] Track {} all 2 attempts failed, caching placeholder",
                    track_id
                );
                let _ = crate::utils::cache::save_artwork_cache(track_id, &[], true);
            }

            // Clear pending flag after completion (success or failure)
            // Note: This runs in background thread, UI state update happens on next repaint

            Ok(())
        })
    });
}
