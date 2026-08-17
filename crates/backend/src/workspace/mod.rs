use std::collections::HashMap;

use calloop::channel::Sender;
use hypr::HyprWorkspaceHandler;
use niri::NiriWorkspaceHandler;

pub mod hypr;
pub mod niri;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceData {
    /// workspace len, start from 1
    pub workspace_count: i32,
    /// index, start from 0
    pub focus: i32,
    /// index, start from 0
    pub active: i32,
}
impl Default for WorkspaceData {
    fn default() -> Self {
        WorkspaceData {
            workspace_count: 1,
            focus: 0,
            active: 0,
        }
    }
}

pub struct WorkspaceCB<T> {
    pub sender: Sender<WorkspaceData>,
    pub output: String,
    pub data: T,
    pub focused_only: bool,
    last_data: Option<WorkspaceData>,
}

impl<T> WorkspaceCB<T> {
    pub fn new(sender: Sender<WorkspaceData>, output: String, data: T, focused_only: bool) -> Self {
        Self {
            sender,
            output,
            data,
            focused_only,
            last_data: None,
        }
    }
}

pub(crate) type ID = u32;

pub(crate) struct WorkspaceCtx<T> {
    id_cache: ID,
    pub(crate) cb: HashMap<ID, WorkspaceCB<T>>,
}

impl<T> WorkspaceCtx<T> {
    pub(crate) fn new() -> Self {
        Self {
            cb: HashMap::new(),
            id_cache: 0,
        }
    }
    pub(crate) fn add_cb(&mut self, cb: WorkspaceCB<T>) -> ID {
        let id = self.id_cache;
        self.cb.insert(id, cb);
        self.id_cache += 1;
        id
    }
    pub(crate) fn remove_cb(&mut self, id: ID) {
        self.cb.remove(&id);
    }
    /// Broadcasts updated workspace data to all registered frontends.
    /// Performs a structural comparison (dirty check) against the last sent data.
    /// If the data is identical, the channel message is suppressed, preventing
    /// unnecessary redraws and achieving 0% CPU usage at idle.
    pub(crate) fn call(&mut self, mut data_func: impl FnMut(&str, &T, bool) -> Option<WorkspaceData>) {
        self.cb.values_mut().for_each(|f| {
            if let Some(data) = data_func(&f.output, &f.data, f.focused_only) {
                // one output should always have a active workspace
                assert!(data.active >= -1);
                // the focus and active workspace should always be the same
                assert!(data.focus < 0 || (data.focus == data.active));
                if f.last_data == Some(data) {
                    return;
                }
                f.sender
                    .send(data)
                    .unwrap_or_else(|e| log::error!("Failed to send workspace data: {}", e));
                f.last_data = Some(data);
            }
        })
    }

    pub(crate) fn sync_all_widgets_unconditionally(
        &self,
        mut data_func: impl FnMut(&str, &T) -> WorkspaceData,
    ) {
        self.cb.values().for_each(|f| {
            let data = data_func(&f.output, &f.data);
            f.sender.send(data).unwrap_or_else(|e| {
                log::error!("Error sending unconditional sync data: {}", e);
            });
        });
    }
}

#[derive(Debug)]
pub enum WorkspaceHandler {
    Hyprland(HyprWorkspaceHandler),
    Niri(NiriWorkspaceHandler),
}
impl WorkspaceHandler {
    pub fn change_to_workspace(&mut self, index: usize) {
        match self {
            WorkspaceHandler::Hyprland(h) => {
                h.change_to_workspace(index);
            }
            WorkspaceHandler::Niri(h) => {
                h.change_to_workspace(index);
            }
        }
    }
}
