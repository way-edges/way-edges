/// Agnostic icon loading and caching module for the Dock widget.
/// This module handles freedesktop icon lookup, SVG rasterization via `resvg`,
/// PNG/JPEG decoding via `image`, and RGBA-to-BGRA pixel format conversion
/// for Cairo compatibility. It is entirely independent of any Window Manager.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use image::imageops::FilterType;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

use crate::runtime::get_backend_runtime_handle;

// ==========================================
// ICON TYPES
// ==========================================

/// Unique cache key for an application icon.
/// Composed of the app_id, desired pixel size, icon theme name, and fallback path.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct IconKey {
    pub app_id: String,
    pub size: u32,
    pub theme: Option<String>,
    pub fallback: Option<String>,
}

/// Represents the current loading state of an icon in the backend pixel cache.
#[derive(Clone, Debug)]
pub enum IconStatus {
    /// Icon lookup/rasterization is in progress on a background thread.
    Loading,
    /// Icon has been successfully rasterized into pre-multiplied BGRA pixels.
    Ready {
        pixels: Arc<[u8]>,
        width: i32,
        height: i32,
        fallback: bool,
    },
    /// Icon could not be found via freedesktop lookup or fallback path.
    NotFound,
}

// ==========================================
// ICON CACHE MANAGEMENT
// ==========================================

/// Checks if a given icon is already in the cache. If not, triggers an
/// asynchronous background load. When the icon finishes loading, `on_ready`
/// is called so that the caller can trigger a redraw notification.
pub fn ensure_icon_loaded(
    icon_cache: &Arc<RwLock<HashMap<IconKey, IconStatus>>>,
    icon_key: IconKey,
    on_ready: impl Fn() + Send + Sync + 'static,
) {
    let needs_load = {
        let cache = icon_cache.read().unwrap();
        !cache.contains_key(&icon_key)
    };

    if needs_load {
        {
            let mut cache = icon_cache.write().unwrap();
            // Double-check after acquiring write lock to avoid duplicate spawns.
            if cache.contains_key(&icon_key) {
                return;
            }
            cache.insert(icon_key.clone(), IconStatus::Loading);
        }
        let cache_clone = Arc::clone(icon_cache);
        get_backend_runtime_handle().spawn(async move {
            load_and_rasterize_icon_async(cache_clone, icon_key).await;
            on_ready();
        });
    }
}

// ==========================================
// ICON LOADING (ASYNC)
// ==========================================

/// Main async entry point for icon loading.
/// Attempts to find the icon by app_id first, then falls back to the configured
/// fallback path if provided. Updates the shared cache with the result.
async fn load_and_rasterize_icon_async(
    cache_arc: Arc<RwLock<HashMap<IconKey, IconStatus>>>,
    icon: IconKey,
) {
    if let Some((pixels, width, height)) = fetch_icon_pixels(&icon).await {
        update_cache_status(
            &cache_arc,
            icon,
            IconStatus::Ready {
                pixels: pixels.into(),
                width,
                height,
                fallback: false,
            },
        );
        return;
    }

    let fallback_str = match &icon.fallback {
        Some(path) if Path::new(path).is_file() => path.clone(),
        _ => {
            update_cache_status(&cache_arc, icon, IconStatus::NotFound);
            return;
        }
    };

    let fb_key = IconKey {
        app_id: fallback_str,
        size: icon.size,
        theme: icon.theme.clone(),
        fallback: icon.fallback.clone(),
    };

    {
        let mut cache = cache_arc.write().unwrap();
        match cache.get(&fb_key) {
            Some(IconStatus::Ready {
                pixels,
                width,
                height,
                ..
            }) => {
                let status = IconStatus::Ready {
                    pixels: pixels.clone(),
                    width: *width,
                    height: *height,
                    fallback: true,
                };
                cache.insert(icon, status);
                return;
            }
            Some(IconStatus::Loading) => {
                cache.insert(icon, IconStatus::Loading);
                return;
            }
            Some(IconStatus::NotFound) => {
                cache.insert(icon, IconStatus::NotFound);
                return;
            }
            None => {
                cache.insert(icon.clone(), IconStatus::Loading);
                cache.insert(fb_key.clone(), IconStatus::Loading);
            }
        }
    }

    match fetch_icon_pixels(&fb_key).await {
        Some((pixels, width, height)) => {
            let status = IconStatus::Ready {
                pixels: pixels.into(),
                width,
                height,
                fallback: true,
            };
            let mut cache = cache_arc.write().unwrap();
            cache.insert(fb_key, status.clone());
            cache.insert(icon, status);
        }
        None => {
            update_cache_status(&cache_arc, icon, IconStatus::NotFound);
        }
    }
}

// ==========================================
// ICON RESOLUTION (FREEDESKTOP)
// ==========================================

/// Searches for a `.desktop` file matching the given app_id and extracts the
/// `Icon=` field value from it.
fn get_icon_from_desktop_file(app_id: &str) -> Option<String> {
    let mut search_paths = vec![
        PathBuf::from(format!("/usr/share/applications/{}.desktop", app_id)),
        PathBuf::from(format!(
            "/usr/local/share/applications/{}.desktop",
            app_id
        )),
    ];

    if let Ok(home) = std::env::var("HOME") {
        search_paths.push(PathBuf::from(format!(
            "{}/.local/share/applications/{}.desktop",
            home, app_id
        )));
    }

    for path in search_paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Icon=") {
                    return Some(trimmed.trim_start_matches("Icon=").to_string());
                }
            }
        }
    }

    None
}

/// Fetches the raw BGRA pixel data for the given icon key.
/// Runs the blocking filesystem I/O and image processing on a dedicated
/// Tokio blocking thread to avoid stalling the async executor.
async fn fetch_icon_pixels(icon: &IconKey) -> Option<(Vec<u8>, i32, i32)> {
    let (app_id, theme, size) = (icon.app_id.to_string(), icon.theme.clone(), icon.size);

    tokio::task::spawn_blocking(move || {
        let path = Path::new(&app_id);
        if path.is_file() {
            if let Some(mut pixels) = process_image_file(path, size) {
                swizzle_rgba_to_bgra(&mut pixels);
                return Some((pixels, size as i32, size as i32));
            }
            return None;
        }

        let (app_id_lower, app_id_short) = generate_search_variants(&app_id);
        let ordered_sizes = generate_ordered_sizes(size);

        let mut names_to_try = Vec::new();

        if let Some(desktop_icon_name) = get_icon_from_desktop_file(&app_id) {
            names_to_try.push(desktop_icon_name);
        }

        for name in [app_id.clone(), app_id_lower, app_id_short] {
            if !names_to_try.contains(&name) {
                names_to_try.push(name);
            }
        }

        for name in &names_to_try {
            for &size_u16 in &ordered_sizes {
                let mut builder = freedesktop_icons::lookup(name).with_size(size_u16);
                if let Some(t) = &theme {
                    if !t.is_empty() {
                        builder = builder.with_theme(t);
                    }
                }
                if let Some(path) = builder.find() {
                    if let Some(mut pixels) = process_image_file(&path, size) {
                        swizzle_rgba_to_bgra(&mut pixels);
                        return Some((pixels, size as i32, size as i32));
                    }
                }
            }
        }
        None
    })
    .await
    .unwrap_or(None)
}

// ==========================================
// IMAGE PROCESSING
// ==========================================

/// Updates a single entry in the icon cache with the given status.
fn update_cache_status(
    cache_arc: &Arc<RwLock<HashMap<IconKey, IconStatus>>>,
    key: IconKey,
    status: IconStatus,
) {
    let mut cache = cache_arc.write().unwrap();
    cache.insert(key, status);
}

/// Dispatches image processing based on file extension (SVG vs raster).
fn process_image_file(path: &Path, size: u32) -> Option<Vec<u8>> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match extension {
        "svg" => rasterize_svg(path, size),
        "png" | "jpg" | "jpeg" => decode_and_resize_raster(path, size),
        _ => None,
    }
}

/// Rasterizes an SVG file to RGBA pixels at the given target size using `resvg`.
fn rasterize_svg(path: &Path, size: u32) -> Option<Vec<u8>> {
    let svg_data = std::fs::read(path).ok()?;
    let opt = Options::default();
    let tree = Tree::from_data(&svg_data, &opt).ok()?;
    let mut pixmap = Pixmap::new(size, size)?;

    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let transform = Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap.take())
}

/// Decodes a raster image (PNG/JPEG) and resizes it to the target size.
/// Pre-multiplies alpha for Cairo compatibility.
fn decode_and_resize_raster(path: &Path, size: u32) -> Option<Vec<u8>> {
    let img = image::open(path).ok()?;
    let resized = img.resize_exact(size, size, FilterType::Lanczos3);
    let mut rgba_img = resized.into_rgba8();

    for pixel in rgba_img.pixels_mut() {
        let alpha = pixel[3] as f32 / 255.0;
        pixel[0] = (pixel[0] as f32 * alpha) as u8;
        pixel[1] = (pixel[1] as f32 * alpha) as u8;
        pixel[2] = (pixel[2] as f32 * alpha) as u8;
    }
    Some(rgba_img.into_raw())
}

/// Converts RGBA pixel data to BGRA format (required by Cairo's ARgb32).
#[inline(always)]
fn swizzle_rgba_to_bgra(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let r = chunk[0];
        let b = chunk[2];
        chunk[0] = b;
        chunk[2] = r;
    }
}

/// Generates lowercase and short (last dot-segment) variants of an app_id
/// for fuzzy icon lookup.
fn generate_search_variants(app_id: &str) -> (String, String) {
    let app_id_lower = app_id.to_lowercase();
    let app_id_short = app_id_lower
        .split('.')
        .last()
        .unwrap_or(&app_id_lower)
        .to_string();
    (app_id_lower, app_id_short)
}

/// Generates an ordered list of standard icon sizes to try, starting from
/// the closest size >= target and then falling back to smaller sizes.
fn generate_ordered_sizes(target_size: u32) -> Vec<u16> {
    const STANDARD_SIZES: [u16; 21] = [
        8, 16, 20, 22, 24, 28, 32, 36, 42, 44, 48, 64, 72, 96, 128, 150, 192, 256, 384, 512,
        1024,
    ];
    let partition_point = STANDARD_SIZES.partition_point(|&x| x < target_size as u16);
    STANDARD_SIZES[partition_point..]
        .iter()
        .chain(STANDARD_SIZES[..partition_point].iter().rev())
        .copied()
        .collect()
}
