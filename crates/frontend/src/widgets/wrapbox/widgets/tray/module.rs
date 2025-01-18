use std::{collections::HashMap, rc::Rc};

use cairo::ImageSurface;
use system_tray::item::StatusNotifierItem;

use config::widgets::wrapbox::tray::TrayConfig;
use way_edges_derive::wrap_rc;

use crate::{mouse_state::MouseEvent, widgets::wrapbox::grid::GridBox};

use super::item::{create_tray_item, TrayID, TrayRc};

#[derive(Debug)]
pub struct TrayModuleState {
    current_mouse_in: Option<TrayRc>,
}
impl TrayModuleState {
    fn new() -> Self {
        Self {
            current_mouse_in: None,
        }
    }
    pub fn set_current_tary(&mut self, tray: TrayRc) -> Option<TrayRc> {
        self.current_mouse_in
            .as_mut()
            .filter(|old| *old != &tray)
            .map(|old| {
                let ret = old.clone();
                *old = tray;
                ret
            })
    }
}

#[wrap_rc(rc = "pub", normal = "pub")]
#[derive(Debug)]
pub struct TrayModule {
    // id
    pub grid: GridBox<TrayRc>,
    pub id_tray_map: HashMap<TrayID, TrayRc>,
    pub module_state: TrayModuleState,
    pub config: Rc<TrayConfig>,
}
impl TrayModule {
    pub fn draw_content(&mut self) -> ImageSurface {
        self.grid.draw()
    }
    pub fn add_tray(&mut self, id: String, tray_item: &StatusNotifierItem) {
        let id = Rc::new(id);

        if self.id_tray_map.contains_key(&id) {
            return;
        }

        let tray = create_tray_item(self, id.clone(), tray_item, self.config.icon_size);
        self.grid.add(tray.clone());
        self.id_tray_map.insert(id, tray);
    }
    pub fn remove_tray(&mut self, id: &String) {
        self.grid.rm(id);
        self.id_tray_map.remove(id);
    }

    pub fn find_tray(&mut self, id: &String) -> Option<TrayRc> {
        self.id_tray_map.get(id).cloned()
    }

    pub fn match_tray_id_from_pos(&self, pos: (f64, f64)) -> Option<(TrayRc, (f64, f64))> {
        self.grid
            .position_map
            .as_ref()
            .unwrap()
            .match_item(pos, &self.grid.item_map)
            .map(|(rc, pos)| (rc.clone(), pos))
    }

    pub fn leave_last_tray(&mut self) -> bool {
        if let Some(f) = self.module_state.current_mouse_in.take() {
            f.borrow_mut().on_mouse_event(MouseEvent::Leave)
        } else {
            false
        }
    }

    pub fn replace_current_tray(&mut self, tray: TrayRc) -> bool {
        if let Some(f) = self.module_state.set_current_tary(tray) {
            f.borrow_mut().on_mouse_event(MouseEvent::Leave)
        } else {
            false
        }
    }
}

pub fn new_tray_module(config: TrayConfig) -> TrayModule {
    let grid = GridBox::new(config.tray_gap as f64, config.grid_align);

    TrayModule {
        grid,
        config: Rc::new(config),
        id_tray_map: HashMap::new(),
        module_state: TrayModuleState::new(),
    }
}

impl GridBox<TrayRc> {
    fn arrangement(num_icons: usize) -> (usize, usize) {
        let num_icons = num_icons as f64;
        let mut best_cols = 1.;
        let mut min_rows = f64::INFINITY;

        let max_col = num_icons.sqrt().ceil();

        let mut cols = 1.;
        while cols <= max_col {
            let rows = (num_icons / cols).ceil();
            if rows < min_rows {
                min_rows = rows;
                best_cols = cols;
            }
            cols += 1.;
        }

        (min_rows as usize, best_cols as usize)
    }

    fn rearrange(&mut self) {
        let len = self.item_map.items.len();

        if len == 0 {
            self.row_col_num = (0, 0);
            self.item_map.row_index.clear();
            return;
        }

        let arrangement = Self::arrangement(len);
        self.row_col_num = arrangement;
        self.item_map.row_index.clear();
        for raw_num in 0..arrangement.0 {
            self.item_map.row_index.push(raw_num * arrangement.1);
        }
    }

    fn add(&mut self, v: TrayRc) {
        self.item_map.items.push(v);
        self.rearrange();
    }

    fn rm(&mut self, v: &str) {
        if let Some(index) = self
            .item_map
            .items
            .iter()
            .position(|tray| tray.borrow().tray_id.as_str() == v)
        {
            self.item_map.items.remove(index);
            self.rearrange();
        }
    }
}
