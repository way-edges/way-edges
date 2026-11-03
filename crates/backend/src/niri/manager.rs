use config::def::widgets::workspace::NiriConf;
use niri_ipc::{Window, Workspace};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once, OnceLock, RwLock};

use crate::dock::icons::{ensure_icon_loaded, IconKey, IconStatus};
use crate::dock::{DockCB, DockData, DockRegistry, DockWindowData, DockWorkspaceData, ID};
use crate::runtime::get_backend_runtime_handle;
use crate::workspace::{WorkspaceCB, WorkspaceCtx, WorkspaceData};

use super::connection::{Connection, Event};

// ==========================================
// DATA CACHE
// ==========================================

/// Central unified cache storing the raw Niri IPC structures.
/// This prevents redundant requests from different widgets (Dock, Workspace).
#[derive(Default, Debug)]
pub struct DataCache {
    pub windows: BTreeMap<u64, Window>,
    pub workspaces: BTreeMap<u64, Workspace>,
}

// ==========================================
// NIRI MANAGER
// ==========================================

/// The Central Niri Hub (Manager).
/// It handles the Niri IPC connection, manages the unified DataCache,
/// owns all widget registries (dock and workspace), and dynamically tracks
/// the demand for windows and workspaces from frontend widgets.
pub struct NiriManager {
    pub cache: RwLock<DataCache>,
    needs_windows: AtomicBool,

    // Widget registries — centralized here to avoid separate unsafe global contexts.
    dock_registry: RwLock<DockRegistry>,
    workspace_registry: RwLock<WorkspaceCtx<NiriConf>>,

    // Icon pixel cache — shared across all dock widgets via Arc.
    icon_cache: Arc<RwLock<HashMap<IconKey, IconStatus>>>,
}

impl NiriManager {
    fn new() -> Self {
        Self {
            cache: RwLock::new(DataCache::default()),
            needs_windows: AtomicBool::new(false),
            dock_registry: RwLock::new(DockRegistry::new()),
            workspace_registry: RwLock::new(WorkspaceCtx::new()),
            icon_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ==========================================
    // DOCK REGISTRATION
    // ==========================================

    /// Registers a new dock widget callback with the centralized registry.
    /// Also flips the `needs_windows` flag and triggers initial data sync.
    pub fn register_dock_callback(&self, cb: DockCB) -> ID {
        let cache = self.cache.write().unwrap();
        let mut registry = self.dock_registry.write().unwrap();
        let cb_id = registry.add_cb(cb);
        let fetch_windows = !self.needs_windows.swap(true, Ordering::Relaxed);
        drop(registry);
        drop(cache);

        if fetch_windows {
            get_backend_runtime_handle().spawn(async move {
                if let Ok(mut conn) = Connection::make_connection().await {
                    if let Ok(Ok(niri_ipc::Response::Windows(wins))) =
                        conn.push_request(niri_ipc::Request::Windows).await
                    {
                        process_event_internal(Event::WindowsChanged { windows: wins }).await;
                    }
                }
            });
        }
        self.notify();
        self.load_icons();

        cb_id
    }

    /// Unregisters a dock widget callback from the centralized registry.
    pub fn unregister_dock_callback(&self, id: ID) {
        let mut cache = self.cache.write().unwrap();
        let mut dock_registry = self.dock_registry.write().unwrap();
        dock_registry.remove_cb(id);
        if dock_registry.is_empty() {
            self.needs_windows.store(false, Ordering::Relaxed);
            cache.windows.clear();
        }
    }

    // ==========================================
    // WORKSPACE REGISTRATION
    // ==========================================

    /// Registers a new workspace widget callback with the centralized registry.
    /// Sends initial data if the cache is already populated.
    pub fn register_workspace_callback(&self, cb: WorkspaceCB<NiriConf>) -> ID {
        let cache = self.cache.read().unwrap();
        if !cache.workspaces.is_empty() {
            let data = get_workspace_data(&cache, &cb.output, cb.data.preserve_empty);
            cb.sender
                .send(data)
                .unwrap_or_else(|e| log::error!("Error sending initial workspace data: {e}"));
        }
        self.workspace_registry.write().unwrap().add_cb(cb)
    }

    /// Unregisters a workspace widget callback from the centralized registry.
    pub fn unregister_workspace_callback(&self, id: ID) {
        self.workspace_registry.write().unwrap().remove_cb(id);
    }

    // ==========================================
    // NOTIFICATIONS (called after IPC events)
    // ==========================================

    /// Notifies all registered widgets (workspaces and docks) of updated Niri IPC state.
    /// Performs a single unified read lock on the cache and groups workspaces/windows
    /// by output, avoiding redundant linear searches and vector allocations.
    pub fn notify(&self) -> Option<DockData> {
        let cache = self.cache.read().unwrap();
        let workspaces_by_output = sort_workspaces_by_outputs(&cache.workspaces);
        let windows_by_output = sort_windows_by_outputs(&cache.workspaces, &cache.windows);

        // 2. Dispatch to Workspaces
        let focused_output = cache
            .workspaces
            .values()
            .find(|w| w.is_focused)
            .and_then(|w| w.output.as_deref());

        self.workspace_registry
            .write()
            .unwrap()
            .call(|output, conf, focused_only| {
                if focused_only {
                    if let Some(focused) = focused_output {
                        if output != focused {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }

                let wps = workspaces_by_output
                    .get(output)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);

                Some(get_workspace_data_from_slice(wps, conf.preserve_empty))
            });

        // Early return in case no dock is active
        if !self.needs_windows.load(Ordering::Relaxed) {
            return None;
        }

        // 5. Dispatch to Docks
        let mut dock_data = DockData::default();
        let mut dock_registry = self.dock_registry.write().unwrap();

        dock_registry.call(|output| {
            let empty_wps = Vec::new();
            let empty_wins = Vec::new();
            let wps = workspaces_by_output.get(output).unwrap_or(&empty_wps);
            let wins = windows_by_output.get(output).unwrap_or(&empty_wins);
            let data = self.build_dock_data(wps, wins);
            dock_data = data.clone();

            (data, false)
        });

        Some(dock_data)
    }

    /// Iterates over all active windows and registered docks to ensure their icons
    /// are asynchronously loaded into the shared cache. Called only on window open/change events.
    pub fn load_icons(&self) {
        let cache = self.cache.read().unwrap();
        let dock_registry = self.dock_registry.read().unwrap();

        let workspace_mapped_to_output = map_workspaces_to_outputs(&cache.workspaces);

        for window in cache.windows.values() {
            if let Some(ws_id) = window.workspace_id {
                if let Some(&window_output) = workspace_mapped_to_output.get(&ws_id) {
                    for cb in dock_registry.iter() {
                        if cb.output == window_output {
                            let app_id = window.app_id.clone().unwrap_or_default();
                            let icon_key = IconKey {
                                app_id,
                                theme: cb.icon_theme.clone(),
                                size: cb.icon_size,
                                fallback: cb.icon_fallback.clone(),
                            };

                            ensure_icon_loaded(&self.icon_cache, icon_key, || {
                                let manager = get_manager();
                                let cache = manager.cache.read().unwrap();
                                manager.dock_registry.write().unwrap().call(|output| {
                                    let wps = sort_workspaces_by_outputs(&cache.workspaces)
                                        .remove(output)
                                        .unwrap_or_default();
                                    let wins =
                                        sort_windows_by_outputs(&cache.workspaces, &cache.windows)
                                            .remove(output)
                                            .unwrap_or_default();

                                    (manager.build_dock_data(&wps, &wins), true)
                                });
                            });
                        }
                    }
                }
            }
        }
    }

    /// Optimized pure translation from pre-filtered slices of Workspaces and Windows.
    fn build_dock_data(
        &self,
        wps_for_output: &[&Workspace],
        wins_for_output: &[&Window],
    ) -> DockData {
        let workspaces: BTreeMap<u64, DockWorkspaceData> = wps_for_output
            .iter()
            .map(|w| {
                (
                    w.id,
                    DockWorkspaceData {
                        id: w.id,
                        idx: w.idx as u64,
                        name: w.name.clone(),
                        output: w.output.clone(),
                        is_active: w.is_active,
                        is_focused: w.is_focused,
                    },
                )
            })
            .collect();

        let windows: Vec<DockWindowData> = wins_for_output
            .iter()
            .map(|w| DockWindowData {
                id: w.id,
                workspace_id: w.workspace_id,
                app_id: w.app_id.clone(),
                title: w.title.clone(),
                is_focused: w.is_focused,
                is_floating: w.is_floating,
                is_urgent: w.is_urgent,
            })
            .collect();

        DockData {
            windows,
            workspaces,
        }
    }

    // ==========================================
    // ACTIONS
    // ==========================================

    /// Pushes an Action payload back to the Niri IPC socket (e.g. FocusWindow, CloseWindow).
    pub fn execute_action(&self, action: niri_ipc::Action) {
        get_backend_runtime_handle().spawn(async move {
            if let Ok(mut conn) = Connection::make_connection().await {
                let _ = conn.push_request(niri_ipc::Request::Action(action)).await;
            }
        });
    }

    /// Returns a clone of the shared icon cache `Arc` for the dock frontend.
    pub fn icon_cache(&self) -> Arc<RwLock<HashMap<IconKey, IconStatus>>> {
        Arc::clone(&self.icon_cache)
    }

    /// Returns a read guard to the workspace registry.
    /// Used by `NiriWorkspaceHandler` to look up callback info for workspace switching.
    pub(crate) fn workspace_registry_ref(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, WorkspaceCtx<NiriConf>> {
        self.workspace_registry.read().unwrap()
    }
}

// ==========================================
// WORKSPACE TRANSLATION (reused from workspace/niri)
// ==========================================

fn filter_empty_workspace<'a>(
    workspaces: impl Iterator<Item = &'a Workspace>,
) -> Vec<&'a Workspace> {
    workspaces
        .filter(|w| w.is_focused || w.active_window_id.is_some() || w.name.is_some())
        .collect()
}

fn get_workspace_data_from_slice(
    wps_for_output: &[&Workspace],
    preserve_empty: bool,
) -> WorkspaceData {
    let v = if preserve_empty {
        wps_for_output.to_vec()
    } else {
        filter_empty_workspace(wps_for_output.iter().copied())
    };

    let focus = v
        .iter()
        .position(|w| w.is_focused)
        .map(|i| i as i32)
        .unwrap_or(-1);
    let active = v
        .iter()
        .position(|w| w.is_active)
        .map(|i| i as i32)
        .unwrap_or(-1);
    let workspace_count = v.len() as i32;

    WorkspaceData {
        workspace_count,
        focus,
        active,
    }
}

fn get_workspace_data(cache: &DataCache, output: &str, preserve_empty: bool) -> WorkspaceData {
    let mut wps_for_output: Vec<&Workspace> = cache
        .workspaces
        .values()
        .filter(|w| w.output.as_deref().unwrap_or_default() == output)
        .collect();

    wps_for_output.sort_by_key(|w| w.idx);

    get_workspace_data_from_slice(&wps_for_output, preserve_empty)
}

fn map_workspaces_to_outputs(workspaces: &BTreeMap<u64, Workspace>) -> HashMap<u64, &str> {
    workspaces
        .iter()
        .map(|(&id, w)| (id, w.output.as_deref().unwrap_or_default()))
        .collect()
}

fn sort_windows_by_outputs<'w>(
    workspaces: &BTreeMap<u64, Workspace>,
    windows: &'w BTreeMap<u64, Window>,
) -> HashMap<String, Vec<&'w Window>> {
    let workspace_to_output = map_workspaces_to_outputs(workspaces);
    let mut windows_by_outputs: HashMap<String, Vec<&Window>> = HashMap::new();

    for window in windows.values() {
        if let Some(ws_id) = window.workspace_id {
            if let Some(&window_output) = workspace_to_output.get(&ws_id) {
                windows_by_outputs
                    .entry(window_output.to_string())
                    .or_default()
                    .push(window);
            }
        }
    }

    // Sorts windows by layout position
    for wins in windows_by_outputs.values_mut() {
        wins.sort_by_key(|w| {
            (
                w.workspace_id
                    .and_then(|id| workspaces.get(&id).map(|ws| ws.idx)),
                w.layout.pos_in_scrolling_layout,
                w.id,
            )
        });
    }

    windows_by_outputs
}

fn sort_workspaces_by_outputs(
    workspaces: &BTreeMap<u64, Workspace>,
) -> HashMap<String, Vec<&Workspace>> {
    let mut workspaces_by_outputs: HashMap<String, Vec<&Workspace>> = HashMap::new();
    for w in workspaces.values() {
        let out = w.output.clone().unwrap_or_default();
        workspaces_by_outputs.entry(out).or_default().push(w);
    }
    for wps in workspaces_by_outputs.values_mut() {
        wps.sort_by_key(|w| w.idx);
    }
    workspaces_by_outputs
}

pub fn get_workspace_by_index<'a>(
    cache: &'a DataCache,
    output: &str,
    preserve_empty: bool,
    index: usize,
) -> Option<&'a Workspace> {
    let mut wps_for_output: Vec<&Workspace> = cache
        .workspaces
        .values()
        .filter(|w| w.output.as_deref().unwrap_or_default() == output)
        .collect();

    wps_for_output.sort_by_key(|w| w.idx);

    let v = if preserve_empty {
        wps_for_output
    } else {
        filter_empty_workspace(wps_for_output.into_iter())
    };

    v.get(index).copied()
}

// ==========================================
// GLOBAL SINGLETON
// ==========================================

static MANAGER: OnceLock<NiriManager> = OnceLock::new();
static LISTENER: Once = Once::new();

pub fn get_manager() -> &'static NiriManager {
    let manager = MANAGER.get_or_init(NiriManager::new);
    LISTENER.call_once(start_listener);
    manager
}

// ==========================================
// IPC LISTENER
// ==========================================

fn start_listener() {
    get_backend_runtime_handle().spawn(async move {
        let mut connection = match Connection::make_connection().await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to connect to Niri IPC on initialization {e}");
                return;
            }
        };

        // Initial workspace sync
        if let Ok(Ok(niri_ipc::Response::Workspaces(ws))) =
            connection.push_request(niri_ipc::Request::Workspaces).await
        {
            let event = Event::WorkspacesChanged { workspaces: ws };
            process_event_internal(event).await;
        }

        // If a dock was registered before the listener started, fetch windows immediately.
        let manager = get_manager();
        if manager.needs_windows.load(Ordering::Relaxed) {
            if let Ok(Ok(niri_ipc::Response::Windows(wins))) =
                connection.push_request(niri_ipc::Request::Windows).await
            {
                let event = Event::WindowsChanged { windows: wins };
                process_event_internal(event).await;
            }
        }

        start_central_listener_loop(connection).await;
    });
}

async fn start_central_listener_loop(connection: Connection) {
    let mut listener = match connection.to_listener().await {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to start Niri EventStream {e}");
            return;
        }
    };

    let mut buf = String::new();

    loop {
        match listener.next_event(&mut buf).await {
            Ok(Some(e)) => {
                process_event_internal(e).await;
            }
            Ok(None) => {}
            Err(err) => {
                log::error!("Error reading Niri EventStream {err}");
                break;
            }
        }
        buf.clear();
    }
}

// ==========================================
// EVENT PROCESSING
// ==========================================

/// Processes raw IPC events.
/// Updates the central cache and notifies the appropriate registries.
/// Conditionally processes window events based on the `needs_windows` flag,
/// avoiding unnecessary CPU load when no Dock widget is active.
pub async fn process_event_internal(e: Event) {
    let manager = get_manager();
    let load_icons = manager
        .cache
        .write()
        .unwrap()
        .apply_event(e, manager.needs_windows.load(Ordering::Relaxed));

    manager.notify();

    if load_icons {
        manager.load_icons();
    }
}

impl DataCache {
    fn apply_event(&mut self, e: Event, needs_windows: bool) -> bool {
        let cache = self;
        let mut load_icons = false;
        match e {
            Event::WorkspacesChanged { workspaces } => {
                cache.workspaces.clear();
                cache
                    .workspaces
                    .extend(workspaces.into_iter().map(|w| (w.id, w)));
                load_icons = needs_windows;
            }
            Event::WorkspaceActivated { id, focused } => {
                if let Some(output) = cache.workspaces.get(&id).map(|w| w.output.clone()) {
                    for (ws_id, ws) in cache.workspaces.iter_mut() {
                        if focused {
                            ws.is_focused = *ws_id == id;
                        }
                        if ws.output == output {
                            ws.is_active = *ws_id == id;
                            if !ws.is_active {
                                ws.is_focused = false;
                            }
                        }
                    }
                }
            }
            Event::WorkspaceActiveWindowChanged {
                workspace_id,
                active_window_id,
            } => {
                if let Some(workspace) = cache.workspaces.get_mut(&workspace_id) {
                    workspace.active_window_id = active_window_id;
                }
            }
            Event::WindowsChanged { windows } => {
                if needs_windows {
                    cache.windows.clear();
                    cache.windows.extend(windows.into_iter().map(|w| (w.id, w)));
                    load_icons = true;
                }
            }
            Event::WindowOpenedOrChanged { window } => {
                if needs_windows {
                    if window.is_focused {
                        cache
                            .windows
                            .values_mut()
                            .for_each(|w| w.is_focused = false);
                    }
                    cache.windows.insert(window.id, window);
                    load_icons = true;
                }
            }
            Event::WindowClosed { id } => {
                if needs_windows {
                    cache.windows.remove(&id);
                }
            }
            Event::WindowFocusChanged { id } => {
                if needs_windows {
                    for (win_id, win) in cache.windows.iter_mut() {
                        win.is_focused = Some(*win_id) == id;
                    }
                }
            }
            Event::WindowUrgencyChanged { id, is_urgent } => {
                if needs_windows {
                    if let Some(win) = cache.windows.get_mut(&id) {
                        if win.is_urgent != is_urgent {
                            win.is_urgent = is_urgent;
                        }
                    }
                }
            }
            Event::WindowLayoutsChanged { changes, .. } => {
                if needs_windows {
                    for (id, new_layout) in changes {
                        if let Some(win) = cache.windows.get_mut(&id) {
                            if win.layout != new_layout {
                                win.layout = new_layout;
                            }
                        }
                    }
                }
            }
        }
        load_icons
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calloop::channel::channel;

    fn workspace(id: u64, idx: u8, output: &str, focused: bool) -> Workspace {
        Workspace {
            id,
            idx,
            name: None,
            output: Some(output.into()),
            is_urgent: false,
            is_active: true,
            is_focused: focused,
            active_window_id: None,
        }
    }

    fn window(id: u64, workspace_id: u64, focused: bool) -> Window {
        Window {
            id,
            title: Some("Window".into()),
            app_id: None,
            pid: None,
            workspace_id: Some(workspace_id),
            is_focused: focused,
            is_floating: false,
            is_urgent: false,
            layout: niri_ipc::WindowLayout {
                pos_in_scrolling_layout: Some((1, 1)),
                tile_size: (100.0, 100.0),
                window_size: (100, 100),
                tile_pos_in_workspace_view: None,
                window_offset_in_tile: (0.0, 0.0),
            },
        }
    }

    #[test]
    fn focus_moves_between_outputs_and_can_leave_all_windows() {
        let mut cache = DataCache::default();
        cache.apply_event(
            Event::WorkspacesChanged {
                workspaces: vec![workspace(1, 1, "A", true), workspace(2, 1, "B", false)],
            },
            true,
        );
        cache.apply_event(
            Event::WindowsChanged {
                windows: vec![window(10, 1, true), window(20, 2, false)],
            },
            true,
        );
        cache.apply_event(
            Event::WorkspaceActivated {
                id: 2,
                focused: true,
            },
            true,
        );
        assert!(!cache.workspaces[&1].is_focused);
        assert!(cache.workspaces[&1].is_active);
        assert!(cache.workspaces[&2].is_focused);
        cache.apply_event(Event::WindowFocusChanged { id: Some(20) }, true);
        assert!(!cache.windows[&10].is_focused);
        assert!(cache.windows[&20].is_focused);
        cache.apply_event(Event::WindowFocusChanged { id: None }, true);
        assert!(cache.windows.values().all(|win| !win.is_focused));
    }

    #[test]
    fn active_window_events_update_empty_workspace_filter_without_a_dock() {
        let mut cache = DataCache::default();
        cache.apply_event(
            Event::WorkspacesChanged {
                workspaces: vec![workspace(1, 1, "A", true), workspace(2, 2, "A", false)],
            },
            false,
        );
        assert_eq!(get_workspace_data(&cache, "A", false).workspace_count, 1);
        cache.apply_event(
            Event::WorkspaceActiveWindowChanged {
                workspace_id: 2,
                active_window_id: Some(10),
            },
            false,
        );
        assert_eq!(get_workspace_data(&cache, "A", false).workspace_count, 2);
        cache.apply_event(
            Event::WorkspaceActiveWindowChanged {
                workspace_id: 2,
                active_window_id: None,
            },
            false,
        );
        assert_eq!(get_workspace_data(&cache, "A", false).workspace_count, 1);
    }

    #[test]
    fn dock_windows_follow_workspace_positions_and_output_assignment() {
        let mut cache = DataCache::default();
        cache.apply_event(
            Event::WorkspacesChanged {
                workspaces: vec![
                    workspace(100, 1, "A", true),
                    workspace(2, 2, "A", false),
                    workspace(3, 1, "B", false),
                ],
            },
            true,
        );
        cache.apply_event(
            Event::WindowsChanged {
                windows: vec![
                    window(1, 2, false),
                    window(2, 100, true),
                    window(3, 3, false),
                ],
            },
            true,
        );
        let by_output = sort_windows_by_outputs(&cache.workspaces, &cache.windows);
        assert_eq!(
            by_output["A"].iter().map(|win| win.id).collect::<Vec<_>>(),
            [2, 1]
        );
        assert_eq!(
            by_output["B"].iter().map(|win| win.id).collect::<Vec<_>>(),
            [3]
        );
    }

    #[test]
    fn focused_only_widgets_receive_initial_state_on_every_output() {
        let manager = NiriManager::new();
        let mut receivers = Vec::new();
        for output in ["A", "B"] {
            let (sender, receiver) = channel();
            manager.register_workspace_callback(WorkspaceCB::new(
                sender,
                output.into(),
                NiriConf {
                    preserve_empty: true,
                },
                true,
            ));
            receivers.push(receiver);
        }
        manager.cache.write().unwrap().apply_event(
            Event::WorkspacesChanged {
                workspaces: vec![workspace(1, 1, "A", true), workspace(2, 1, "B", false)],
            },
            false,
        );
        manager.notify();
        assert_eq!(receivers[0].try_recv().unwrap().focus, 0);
        assert_eq!(receivers[1].try_recv().unwrap().focus, -1);
        manager.notify();
        assert!(receivers
            .iter()
            .all(|receiver| receiver.try_recv().is_err()));
    }

    #[test]
    fn urgency_events_use_the_niri_wire_field() {
        let mut cache = DataCache::default();
        cache.windows.insert(1, window(1, 1, false));
        let event: Event =
            serde_jsonrc::from_str(r#"{"WindowUrgencyChanged":{"id":1,"urgent":true}}"#).unwrap();
        cache.apply_event(event, true);
        assert!(cache.windows[&1].is_urgent);
    }
}
