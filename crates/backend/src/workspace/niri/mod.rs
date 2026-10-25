/// Niri-specific adapter for the Workspace widget.
/// This module is now a thin delegation layer: it delegates all registration
/// and data translation to the centralized `NiriManager`.
/// Only the WM-specific action handler (change_to_workspace) remains here.
use config::def::widgets::workspace::NiriConf;

use crate::niri::manager::{get_manager, get_workspace_by_index};
use crate::runtime::get_backend_runtime_handle;
use super::{WorkspaceCB, WorkspaceHandler, ID};

// ==========================================
// PUBLIC API
// ==========================================

/// Registers a new workspace widget callback by delegating to the centralized NiriManager.
/// Returns a `WorkspaceHandler` that the frontend uses for workspace switching actions.
pub fn register_niri_event_callback(cb: WorkspaceCB<NiriConf>) -> WorkspaceHandler {
    let manager = get_manager();
    let cb_id = manager.register_workspace_callback(cb);
    WorkspaceHandler::Niri(NiriWorkspaceHandler { cb_id })
}

/// Unregisters a workspace widget callback from the centralized NiriManager.
pub fn unregister_niri_event_callback(id: ID) {
    get_manager().unregister_workspace_callback(id);
}

// ==========================================
// HANDLER
// ==========================================

/// Niri-specific implementation of the WorkspaceHandler.
/// Converts agnostic commands into `niri_ipc::Action::FocusWorkspace` via the Central Hub.
/// Automatically unregisters from the NiriManager when dropped.
#[derive(Debug)]
pub struct NiriWorkspaceHandler {
    cb_id: ID,
}

impl Drop for NiriWorkspaceHandler {
    fn drop(&mut self) {
        unregister_niri_event_callback(self.cb_id);
    }
}

impl NiriWorkspaceHandler {
    pub fn change_to_workspace(&mut self, index: usize) {
        let cb_id = self.cb_id;
        get_backend_runtime_handle().spawn(async move {
            let manager = get_manager();

            let (output, preserve_empty) = {
                let ws_registry = manager.workspace_registry_ref();
                match ws_registry.cb.get(&cb_id).map(|w| (w.output.clone(), w.data.preserve_empty)) {
                    Some(v) => v,
                    None => return,
                }
            };

            let cache = manager.cache.read().unwrap();

            let Some(id) =
                get_workspace_by_index(&cache, &output, preserve_empty, index).map(|w| w.id)
            else {
                return;
            };

            manager.execute_action(niri_ipc::Action::FocusWorkspace {
                reference: niri_ipc::WorkspaceReferenceArg::Id(id),
            });
        });
    }
}
