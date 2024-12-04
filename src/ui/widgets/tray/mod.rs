use std::{cell::RefCell, collections::HashMap, rc::Rc};

use cairo::ImageSurface;

use crate::{
    config::widgets::wrapbox::Align,
    plug::tray::{register_tray, unregister_tray, TrayIcon, TrayItem, TrayMenu},
};

use super::wrapbox::{
    display::grid::{DisplayWidget, GridBox},
    expose::BoxExpose,
};

struct Menu {}

struct Tray {
    tray_id: TrayID,
    id: String,
    title: Option<String>,
    icon: ImageSurface,
    menu_path: Option<String>,

    menu: Option<Menu>,
}

impl Tray {
    fn update_title(&mut self, title: Option<String>) {}
    fn update_icon(&mut self, icon: TrayIcon) {}
    fn update_menu(&mut self, menu: Menu) {}
}

impl GridBox<TrayID> {
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
        let len = self.map.items.len();

        if len == 0 {
            self.row_col_num = (0, 0);
            self.map.row_index.clear();
            return;
        }

        let arrangement = Self::arrangement(len);
        self.row_col_num = arrangement;
        self.map.row_index.clear();
        for raw_num in 0..arrangement.0 {
            self.map.row_index.push(raw_num * arrangement.1);
        }
    }

    fn add(&mut self, v: TrayID) {
        self.map.items.push(v);
        self.rearrange();
    }

    fn rm(&mut self, v: &str) {
        if let Some(index) = self.map.items.iter().position(|id| id.as_str() == v) {
            self.map.items.remove(index);
            self.rearrange();
        }
    }
}

type TrayID = Rc<String>;

struct TrayModule {
    // id
    grid: GridBox<TrayID>,
    id_tray_map: HashMap<TrayID, Tray>,
}
impl TrayModule {
    fn new() -> Self {
        let grid = GridBox::new(0., Align::CenterCenter);
        Self {
            grid,
            id_tray_map: HashMap::new(),
        }
    }
    fn add_tray(&mut self, id: String, tray_item: &TrayItem) {
        let id = Rc::new(id);
        let tray = Self::parse_tary_item(id.clone(), tray_item, 16);

        self.grid.add(id.clone());
        self.id_tray_map.insert(id, tray);
    }
    fn remove_tray(&mut self, id: &String) {
        self.grid.rm(id);
        self.id_tray_map.remove(id);
    }

    fn parse_tray_icon(value: &TrayIcon, size: i32) -> ImageSurface {
        value
            .get_icon_with_size(size, 1)
            .unwrap_or(ImageSurface::create(cairo::Format::ARgb32, size, size).unwrap())
    }
    fn parse_tary_item(tray_id: TrayID, value: &TrayItem, size: i32) -> Tray {
        let id = value.id.clone();
        let title = value.title.clone();
        let icon = Self::parse_tray_icon(&value.icon, size);
        let menu_path = value.menu_path.clone();
        Tray {
            tray_id,
            id,
            title,
            icon,
            menu_path,
            menu: None,
        }
    }
}

struct TrayCtx {
    module: TrayModule,
    backend_cb_id: i32,
    content: cairo::ImageSurface,
}
impl Default for TrayCtx {
    fn default() -> Self {
        Self {
            module: TrayModule::new(),
            backend_cb_id: Default::default(),
            content: ImageSurface::create(cairo::Format::ARgb32, 0, 0).unwrap(),
        }
    }
}
impl Drop for TrayCtx {
    fn drop(&mut self) {
        unregister_tray(self.backend_cb_id);
    }
}

impl DisplayWidget for TrayCtx {
    fn get_size(&mut self) -> (f64, f64) {
        todo!()
    }

    fn content(&mut self) -> cairo::ImageSurface {
        todo!()
    }

    fn on_mouse_event(&mut self, _: crate::ui::draws::mouse_state::MouseEvent) {}
}

pub fn init_tray(expose: &BoxExpose) {
    use gtk::glib;

    let update_func = expose.update_func();

    let ctx = Rc::new(RefCell::new(TrayCtx::default()));

    let backend_cb_id = register_tray(Box::new(glib::clone!(
        #[weak]
        ctx,
        move |(id, e)| {
            let mut ctx = ctx.borrow_mut();
            use crate::plug::tray::Event;
            match e {
                Event::ItemNew(tray_item) => {
                    ctx.module.add_tray(id.clone(), tray_item);
                }
                Event::ItemRemove => {
                    ctx.module.remove_tray(id);
                }
                _ => {} // Event::TitleUpdate(title) => todo!(),
                        // Event::IconUpdate(tray_icon) => todo!(),
                        // Event::MenuNew(tray_menu) => todo!(),
            }
        }
    )));
}
