pub mod draw;
pub mod layout;

use crate::mouse_state::{MouseEvent, MouseStateData};
use crate::wayland::app::WidgetBuilder;
use backend::dock::icons::{IconKey, IconStatus};
use backend::dock::{DockCB, DockData, DockHandler};
use cairo::{Format, ImageSurface};
use config::def::widgets::dock::DockConfig;
use cosmic_text::{FontSystem, SwashCache};
use smithay_client_toolkit::output::OutputInfo;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::WidgetContext;
use layout::DockLayout;

#[derive(Debug)]
pub struct DockCtx {
    pub config: DockConfig,
    pub last_layout: DockLayout,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub icon_surface_cache: HashMap<IconKey, ImageSurface>,
    pub glyph_cache: HashMap<cosmic_text::CacheKey, ImageSurface>,
    pub dock_data: Rc<RefCell<DockData>>,
    pub handler: DockHandler,
    pub is_vertical: bool,
    pub target_focus_id: Option<u64>,
}

impl DockCtx {
    /// RECONCILIATION: Transforms the backend cache (`DockHandler.icon_cache`)
    /// into Cairo `ImageSurface`s and populates the frontend cache (`icon_surface_cache`).
    /// This keeps the UI rendering detached from backend image fetching logic.
    fn reconcile_icons(&mut self, windows: &[backend::dock::DockWindowData]) {
        let backend_cache = self.handler.icon_cache().read().unwrap();
        let config = &self.config.windows;

        for win in windows {
            let app_id = win.app_id.clone().unwrap_or_default();
            let key = IconKey {
                app_id,
                theme: config.icon_theme.clone(),
                size: config.icon_size as u32,
                fallback: config.icon_fallback.clone(),
            };

            if self.icon_surface_cache.contains_key(&key) {
                continue;
            }

            if let Some(IconStatus::Ready {
                pixels,
                width,
                height,
                ..
            }) = backend_cache.get(&key)
            {
                if let Ok(mut surface) = ImageSurface::create(Format::ARgb32, *width, *height) {
                    let stride = surface.stride() as usize;
                    if let Ok(mut data) = surface.data() {
                        let w_bytes = (*width * 4) as usize;
                        for (dest_row, src_row) in data
                            .chunks_exact_mut(stride)
                            .zip(pixels.chunks_exact(w_bytes))
                        {
                            dest_row[..w_bytes].copy_from_slice(src_row);
                        }
                    }
                    surface.mark_dirty();
                    self.icon_surface_cache.insert(key, surface);
                }
            }
        }
    }
}

impl WidgetContext for DockCtx {
    fn redraw(&mut self) -> ImageSurface {
        let dock_data = self.dock_data.borrow().clone();

        self.reconcile_icons(&dock_data.windows);

        // Calculate the positions and dimensions of all items based on agnostic data.
        // The layout module does not know about Niri.
        self.last_layout = DockLayout::calculate(
            &dock_data,
            &self.config,
            &mut self.icon_surface_cache,
            &mut self.font_system,
            self.is_vertical,
        );

        // Allocate the main ImageSurface for Cairo rendering.
        let width = self.last_layout.rect.width().max(1.0) as i32;
        let height = self.last_layout.rect.height().max(1.0) as i32;
        let surface = ImageSurface::create(Format::ARgb32, width, height)
            .expect("Falha ao alocar buffer Cairo para a Dock");

        // Draws the dock
        draw::paint(
            &surface,
            &self.last_layout,
            &self.config,
            &self.icon_surface_cache,
            &mut self.font_system,
            &mut self.swash_cache,
            &mut self.glyph_cache,
        );

        surface
    }

    fn on_mouse_event(&mut self, _data: &MouseStateData, event: MouseEvent) -> bool {
        match event {
            MouseEvent::Press((x, y), button) => {
                let is_left = button == 272;
                let is_middle = button == 274;

                if let Some(item) = self.last_layout.find_clicked_item(x, y) {
                    if is_left {
                        self.handler.focus_window(item.id as u64);
                        return true;
                    } else if is_middle {
                        self.handler.close_window(item.id as u64);
                        return true;
                    }
                }
                false
            }
            MouseEvent::Scroll(h, v) => {
                let mut delta = v.discrete;
                if delta == 0 {
                    delta = h.discrete;
                }
                if delta == 0 {
                    // Fallback to absolute for smooth scrolling (touchpads)
                    if v.absolute > 5.0 || h.absolute > 5.0 {
                        delta = 1;
                    } else if v.absolute < -5.0 || h.absolute < -5.0 {
                        delta = -1;
                    } else {
                        return false;
                    }
                }

                let dock_data = self.dock_data.borrow();
                let windows = &dock_data.windows;

                if windows.is_empty() {
                    return false;
                }

                // If we are scrolling rapidly, the backend might not have updated `w.is_focused` yet.
                // We track our local `target_focus_id` to calculate the correct consecutive next/prev window.
                let focused_idx = if let Some(target_id) = self.target_focus_id {
                    windows
                        .iter()
                        .position(|w| w.id == target_id)
                        .unwrap_or_else(|| windows.iter().position(|w| w.is_focused).unwrap_or(0))
                } else {
                    windows.iter().position(|w| w.is_focused).unwrap_or(0)
                } as i32;

                let new_idx = (focused_idx + delta).rem_euclid(windows.len() as i32);
                let window_id = windows[new_idx as usize].id;

                self.target_focus_id = Some(window_id);
                drop(dock_data);
                self.handler.focus_window(window_id);
                return true;
            }
            _ => false,
        }
    }
}

/// Initializes the Dock Widget state context.
/// Establishes the communication channel (`calloop`) with the agnostic backend.
pub fn init_widget(
    builder: &mut WidgetBuilder,
    _size: (i32, i32),
    config: DockConfig,
    output: &OutputInfo,
) -> impl WidgetContext {
    let output_name = output.name.clone().unwrap_or_default();

    let edge = builder.common_config.edge;
    let is_vertical = edge.contains(smithay_client_toolkit::shell::wlr_layer::Anchor::LEFT)
        || edge.contains(smithay_client_toolkit::shell::wlr_layer::Anchor::RIGHT);

    let dock_data = Rc::new(RefCell::new(DockData::default()));
    let dock_data_weak = Rc::downgrade(&dock_data);

    // `make_redraw_channel` creates a `calloop::channel` internally.
    // The provided closure runs in the main Wayland Event Loop when data is pushed from the backend.
    // We update the local `RefCell` so that `WidgetContext::redraw()` can draw the new state.
    let sender = builder.make_redraw_channel(move |_app, msg: DockData| {
        if let Some(data) = dock_data_weak.upgrade() {
            *data.borrow_mut() = msg;
        }
    });

    let cb = DockCB::new(
        sender,
        output_name,
        config.windows.icon_size as u32,
        config.windows.icon_theme.clone(),
        config.windows.icon_fallback.clone(),
    );
    let handler = backend::dock::niri::register_dock_callback(cb);

    DockCtx {
        config,
        last_layout: DockLayout::new(),
        font_system: cosmic_text::FontSystem::new(),
        swash_cache: cosmic_text::SwashCache::new(),
        icon_surface_cache: HashMap::new(),
        glyph_cache: HashMap::new(),
        dock_data,
        handler,
        is_vertical,
        target_focus_id: None,
    }
}
