use calloop::channel::Sender;
use std::collections::{BTreeMap, HashMap};

pub mod icons;
pub mod niri;

/// Agnostic representation of a window in the dock.
/// This struct is independent of the underlying Window Manager (e.g., Niri, Hyprland).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockWindowData {
    pub id: u64,
    pub workspace_id: Option<u64>,
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub is_focused: bool,
    pub is_floating: bool,
    pub is_urgent: bool,
}

/// Agnostic representation of a workspace in the dock.
/// This struct is independent of the underlying Window Manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockWorkspaceData {
    pub id: u64,
    pub idx: u64,
    pub name: Option<String>,
    pub output: Option<String>,
    pub is_active: bool,
    pub is_focused: bool,
}

/// Consolidated agnostic data payload sent to the dock frontend.
/// Contains pre-filtered and pre-sorted windows and workspaces for a specific output.
/// Derives `PartialEq` and `Eq` to enable structural dirty-checking in the backend,
/// preventing redundant redraws when the data hasn't actually changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DockData {
    pub windows: Vec<DockWindowData>,
    pub workspaces: BTreeMap<u64, DockWorkspaceData>,
}

/// Callback payload storing the communication channel to the frontend
/// and the specific output monitor this dock is tied to.
/// Also stores the last data sent to enable dirty-checking.
pub struct DockCB {
    pub sender: Sender<DockData>,
    pub output: String,
    pub icon_size: u32,
    pub icon_theme: Option<String>,
    pub icon_fallback: Option<String>,
    last_data: Option<DockData>,
}

impl DockCB {
    pub fn new(
        sender: Sender<DockData>,
        output: String,
        icon_size: u32,
        icon_theme: Option<String>,
        icon_fallback: Option<String>,
    ) -> Self {
        Self {
            sender,
            output,
            icon_size,
            icon_theme,
            icon_fallback,
            last_data: None,
        }
    }
}

pub(crate) type ID = u32;

/// Registry that manages all active dock frontend callbacks.
/// This allows the backend to broadcast updates to all active dock widgets.
/// Uses structural dirty-checking (`PartialEq`) to avoid sending redundant
/// updates that would cause unnecessary CPU usage in the frontend.
pub(crate) struct DockRegistry {
    id_cache: ID,
    cb: HashMap<ID, DockCB>,
}

impl DockRegistry {
    pub fn new() -> Self {
        Self {
            cb: HashMap::new(),
            id_cache: 0,
        }
    }
    pub fn add_cb(&mut self, cb: DockCB) -> ID {
        let id = self.id_cache;
        self.cb.insert(id, cb);
        self.id_cache += 1;
        id
    }
    pub fn remove_cb(&mut self, id: ID) {
        self.cb.remove(&id);
    }
    pub fn is_empty(&self) -> bool {
        self.cb.is_empty()
    }
    pub fn iter(&self) -> std::collections::hash_map::Values<'_, ID, DockCB> {
        self.cb.values()
    }
    /// Broadcasts updated dock data to all registered frontends.
    /// Performs a structural comparison (dirty check) against the last sent data.
    /// If the data is identical, the channel message is suppressed, preventing
    /// unnecessary redraws and achieving 0% CPU usage at idle.
    pub fn call(&mut self, mut data_func: impl FnMut(&str) -> (DockData, bool)) {
        self.cb.values_mut().for_each(|f| {
            let (data, force) = data_func(&f.output);
            if !force && f.last_data.as_ref() == Some(&data) {
                return;
            }
            f.sender
                .send(data.clone())
                .unwrap_or_else(|e| log::error!("Failed to send dock data: {}", e));
            f.last_data = Some(data);
        });
    }
}

/// Abstraction for Window Manager specific actions (e.g., focus, close).
/// Allows the agnostic frontend to trigger WM-specific IPC commands.
#[derive(Debug)]
pub enum DockHandler {
    Niri(niri::NiriDockHandler),
}

impl DockHandler {
    pub fn focus_window(&self, window_id: u64) {
        match self {
            DockHandler::Niri(h) => h.focus_window(window_id),
        }
    }
    pub fn close_window(&self, window_id: u64) {
        match self {
            DockHandler::Niri(h) => h.close_window(window_id),
        }
    }
    /// Returns a reference to the shared icon pixel cache.
    /// The cache is managed by the agnostic `icons` module and shared across
    /// all dock widgets via `Arc`.
    pub fn icon_cache(
        &self,
    ) -> &std::sync::Arc<std::sync::RwLock<HashMap<icons::IconKey, icons::IconStatus>>> {
        match self {
            DockHandler::Niri(h) => &h.icon_cache,
        }
    }
}
