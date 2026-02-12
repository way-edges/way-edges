use std::{collections::HashMap, ops::Deref, sync::Arc};

use backend::tray::{item::Tray, TrayMap};
use cairo::ImageSurface;

use config::widgets::wrapbox::tray::TrayConfig;

use crate::{mouse_state::MouseEvent, widgets::wrapbox::grid::GridBox};

use super::item::TrayState;

#[derive(Debug)]
pub struct ModuleState {
    current_mouse_in: Option<Destination>,
}
impl ModuleState {
    fn new() -> Self {
        Self {
            current_mouse_in: None,
        }
    }
    pub fn set_current_tary(&mut self, dest: Destination) -> Option<Destination> {
        self.current_mouse_in
            .as_mut()
            .filter(|old| *old != &dest)
            .map(|old| {
                let ret = old.clone();
                *old = dest;
                ret
            })
    }
}

type Destination = Arc<String>;

#[derive(Debug)]
pub struct TrayModule {
    // id
    pub grid: GridBox<Destination>,
    pub id_tray_map: HashMap<Destination, TrayState>,
    pub module_state: ModuleState,
    pub config: TrayConfig,
}
impl TrayModule {
    pub fn draw_content(&mut self, tray_map: &TrayMap) -> ImageSurface {
        self.grid.draw(|dest| {
            let tray_state = self.id_tray_map.get_mut(dest).unwrap();
            let tray = tray_map.get(dest).unwrap().lock().unwrap();
            tray_state.draw(tray.deref(), &self.config)
        })
    }
    pub fn add_tray(&mut self, dest: Arc<String>, tray: &Tray) {
        if self.id_tray_map.contains_key(&dest) {
            return;
        }

        let state = TrayState::new(dest.clone(), tray);

        self.grid.add(dest.clone());
        self.id_tray_map.insert(dest, state);
    }
    pub fn remove_tray(&mut self, id: &String) {
        self.grid.rm(id);
        self.id_tray_map.remove(id);
    }

    pub fn update_tray(&mut self, dest: &String, tray: &Tray) {
        if let Some(state) = self.find_tray(dest) {
            state.update_tray(tray)
        }
    }

    pub fn find_tray(&mut self, id: &String) -> Option<&mut TrayState> {
        self.id_tray_map.get_mut(id)
    }

    pub fn match_tray_id_from_pos(
        &mut self,
        pos: (f64, f64),
    ) -> Option<(Arc<String>, &mut TrayState, (f64, f64))> {
        let (dest, pos) = self
            .grid
            .position_map
            .as_ref()
            .unwrap()
            .match_item(pos, &self.grid.item_map)?;

        let dest = dest.clone();
        self.find_tray(&dest).map(|state| (dest, state, pos))
    }

    pub fn leave_last_tray(&mut self) -> bool {
        if let Some(f) = self.module_state.current_mouse_in.take() {
            self.find_tray(&f)
                .map(|state| state.on_mouse_event(MouseEvent::Leave))
                .unwrap_or_default()
        } else {
            false
        }
    }

    pub fn replace_current_tray(&mut self, dest: Destination) -> bool {
        if let Some(f) = self.module_state.set_current_tary(dest) {
            self.find_tray(&f)
                .map(|state| state.on_mouse_event(MouseEvent::Leave))
                .unwrap_or_default()
        } else {
            false
        }
    }
}

pub fn new_tray_module(config: TrayConfig) -> TrayModule {
    let grid = GridBox::new(config.tray_gap as f64, config.grid_align);

    TrayModule {
        grid,
        config,
        id_tray_map: HashMap::new(),
        module_state: ModuleState::new(),
    }
}

impl GridBox<Destination> {
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

    fn add(&mut self, v: Arc<String>) {
        self.item_map.items.push(v);
        self.rearrange();
    }

    fn rm(&mut self, v: &str) {
        if let Some(index) = self
            .item_map
            .items
            .iter()
            .position(|tray| tray.as_str() == v)
        {
            self.item_map.items.remove(index);
            self.rearrange();
        }
    }
}
