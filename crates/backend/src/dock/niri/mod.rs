/// Niri-specific adapter for the Dock widget.
/// This module is now a thin delegation layer: it delegates all registration,
/// data translation, and icon management to the centralized `NiriManager`.
/// Only the WM-specific action handlers (focus/close window) remain here.
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::dock::icons::{IconKey, IconStatus};
use crate::niri::manager::get_manager;
use super::{DockCB, DockHandler, ID};

// ==========================================
// PUBLIC API
// ==========================================

/// Registers a new dock widget callback by delegating to the centralized NiriManager.
/// Returns a `DockHandler` that the frontend uses for window actions and icon cache access.
pub fn register_dock_callback(cb: DockCB) -> DockHandler {
    let manager = get_manager();
    let icon_cache = manager.icon_cache();
    let cb_id = manager.register_dock_callback(cb);
    DockHandler::Niri(NiriDockHandler { cb_id, icon_cache })
}

/// Unregisters a dock widget callback from the centralized NiriManager.
pub fn unregister_dock_callback(id: ID) {
    get_manager().unregister_dock_callback(id);
}

// ==========================================
// HANDLER
// ==========================================

/// Niri-specific implementation of the DockHandler.
/// Provides methods to trigger Window Manager actions (Focus/Close) via the Central Hub,
/// and automatically unregisters callbacks when the widget is dropped.
#[derive(Debug)]
pub struct NiriDockHandler {
    cb_id: ID,
    pub icon_cache: Arc<RwLock<HashMap<IconKey, IconStatus>>>,
}

impl Drop for NiriDockHandler {
    fn drop(&mut self) {
        unregister_dock_callback(self.cb_id);
    }
}

impl NiriDockHandler {
    pub fn focus_window(&self, window_id: u64) {
        let manager = get_manager();
        manager.execute_action(niri_ipc::Action::FocusWindow { id: window_id });
    }

    pub fn close_window(&self, window_id: u64) {
        let manager = get_manager();
        manager.execute_action(niri_ipc::Action::CloseWindow { id: Some(window_id) });
    }
}
