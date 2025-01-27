use std::collections::HashMap;

use calloop::channel::Sender;
use hypr::HyprWorkspaceHandler;
use niri::NiriWorkspaceHandler;

pub mod hypr;
pub mod niri;

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceData {
    /// workspace len, start from 1
    pub workspace_count: i32,
    /// index, start from 0
    pub focus: i32,
}
impl Default for WorkspaceData {
    fn default() -> Self {
        WorkspaceData {
            workspace_count: 1,
            focus: 0,
        }
    }
}

pub struct WorkspaceCB {
    sender: Sender<WorkspaceData>,
    output: String,
}

type ID = u32;

struct WorkspaceCtx {
    id_cache: ID,
    cb: HashMap<ID, WorkspaceCB>,
}

impl WorkspaceCtx {
    fn new() -> Self {
        Self {
            cb: HashMap::new(),
            id_cache: 0,
        }
    }
    fn add_cb(&mut self, cb: WorkspaceCB) -> ID {
        let id = self.id_cache;
        self.cb.insert(id, cb);
        self.id_cache += 1;
        id
    }
    fn remove_cb(&mut self, id: ID) {
        self.cb.remove(&id);
    }
    fn call(&mut self, mut data_func: impl FnMut(&str) -> WorkspaceData) {
        self.cb.values_mut().for_each(|f| {
            f.sender.send(data_func(&f.output)).unwrap();
        })
    }
}

#[derive(Debug)]
pub enum WorkspaceHandler {
    Hyprland(HyprWorkspaceHandler),
    Niri(NiriWorkspaceHandler),
}
impl WorkspaceHandler {
    pub fn change_to_workspace(&mut self, output: &str, index: usize) {
        match self {
            WorkspaceHandler::Hyprland(h) => {
                h.change_to_workspace(output, index);
            }
            WorkspaceHandler::Niri(h) => {
                h.change_to_workspace(output, index);
            }
        }
    }
}
