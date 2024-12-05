use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use cairo::ImageSurface;

use crate::{
    config::widgets::wrapbox::Align,
    plug::{
        self,
        tray::{register_tray, unregister_tray, TrayIcon, TrayItem, TrayMenu},
    },
};

use super::wrapbox::{
    display::grid::{DisplayWidget, GridBox},
    expose::{BoxExpose, BoxRedrawFunc},
};

struct MenuState {
    open_state: HashSet<i32>,
    hover_state: i32,
}
impl MenuState {
    fn filter_state_with_new_menu(&mut self, menu: &RootMenu) {}
}

struct RootMenu {
    id: i32,
    submenus: Vec<Menu>,
}

struct CommonMenu {
    pub id: i32,
    pub label: Option<String>,
    pub enabled: bool,
    pub icon: Option<TrayIcon>,
}

enum Menu {
    Radio {
        common: CommonMenu,
        choosed: bool,
    },
    Check {
        common: CommonMenu,
        checked: bool,
    },
    WithSubmenu {
        common: CommonMenu,
        submenu: Vec<Menu>,
    },
    Normal(CommonMenu),
    Separator,
}

struct Tray {
    tray_id: TrayID,
    id: String,
    title: Option<String>,
    icon: ImageSurface,
    menu_path: Option<String>,

    menu: Option<(RootMenu, MenuState)>,

    updated: bool,
    content: ImageSurface,
}

impl Tray {
    fn update_title(&mut self, title: Option<String>) {
        if title != self.title {
            self.title = title;
            self.set_updated();
        }
    }
    fn update_icon(&mut self, icon: ImageSurface) {
        self.icon = icon;
        self.set_updated();
    }
    fn update_menu(&mut self, new: RootMenu) {
        if let Some((old, state)) = &mut self.menu {
            state.filter_state_with_new_menu(&new);
            *old = new;
        }
    }
}
impl Tray {
    fn set_updated(&mut self) {
        self.updated = true;
    }
    fn redraw_if_updated(&mut self) {
        if self.updated {
            self.draw();
            self.updated = false;
        }
    }
    fn draw(&mut self) {}
}
impl DisplayWidget for Tray {
    fn get_size(&self) -> (f64, f64) {
        (self.icon.width() as f64, self.icon.height() as f64)
    }

    fn content(&self) -> ImageSurface {
        self.icon.clone()
    }

    fn on_mouse_event(&mut self, _: crate::ui::draws::mouse_state::MouseEvent) {}
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
    redraw_signal: BoxRedrawFunc,

    icon_size: i32,
}
impl TrayModule {
    fn draw_content(&mut self) -> ImageSurface {
        self.grid.draw(
            |id| self.id_tray_map.get(id).unwrap().get_size(),
            |id| self.id_tray_map.get(id).unwrap().content(),
        )
    }
    fn new(redraw_signal: BoxRedrawFunc) -> Self {
        let grid = GridBox::new(0., Align::CenterCenter);
        Self {
            grid,
            id_tray_map: HashMap::new(),
            redraw_signal,

            icon_size: 16,
        }
    }
    fn add_tray(&mut self, id: String, tray_item: &TrayItem) {
        let id = Rc::new(id);
        let tray = Self::parse_tary_item(id.clone(), tray_item, self.icon_size);

        self.grid.add(id.clone());
        self.id_tray_map.insert(id, tray);

        (self.redraw_signal)()
    }
    fn remove_tray(&mut self, id: &String) {
        self.grid.rm(id);
        self.id_tray_map.remove(id);

        (self.redraw_signal)()
    }

    fn find_tray(&mut self, id: &String) -> Option<&mut Tray> {
        self.id_tray_map.get_mut(id)
    }

    fn parse_tray_icon(value: &TrayIcon, size: i32) -> ImageSurface {
        value
            .get_icon_with_size(size)
            .unwrap_or(ImageSurface::create(cairo::Format::ARgb32, size, size).unwrap())
    }
    fn parse_tary_item(tray_id: TrayID, value: &TrayItem, icon_size: i32) -> Tray {
        let id = value.id.clone();
        let title = value.title.clone();
        let icon = Self::parse_tray_icon(&value.icon, icon_size);
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
    fn parse_menu(menu: &plug::tray::Menu, icon_size: i32) -> Menu {}
    fn parse_tray_menu(tray_menu: &TrayMenu, icon_size: i32) -> RootMenu {
        let id = tray_menu.id as i32;
        let submenus = tray_menu
            .menus
            .iter()
            .map(|menu| TrayModule::parse_menu(menu, icon_size))
            .collect();
        RootMenu { id, submenus }
    }
}

pub struct TrayCtx {
    module: TrayModule,
    backend_cb_id: i32,
    content: cairo::ImageSurface,
}
impl TrayCtx {
    fn new(module: TrayModule) -> Self {
        Self {
            module,
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
    fn get_size(&self) -> (f64, f64) {
        (self.content.width() as f64, self.content.height() as f64)
    }

    fn content(&self) -> cairo::ImageSurface {
        self.content.clone()
    }

    fn on_mouse_event(&mut self, _: crate::ui::draws::mouse_state::MouseEvent) {}
}

pub fn init_tray(expose: &BoxExpose) -> Rc<RefCell<TrayCtx>> {
    use gtk::glib;

    let ctx = Rc::<RefCell<TrayCtx>>::new_cyclic(|me| {
        // make module
        let update_func = expose.update_func();
        let me = me.clone();
        let tray_redraw_func = Rc::new(move || {
            if let Some(ctx) = me.upgrade() {
                let ctx = unsafe { ctx.as_ptr().as_mut() }.unwrap();
                ctx.content = ctx.module.draw_content();
                update_func();
            }
        });
        let module = TrayModule::new(tray_redraw_func);

        RefCell::new(TrayCtx::new(module))
    });

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
                Event::TitleUpdate(title) => {
                    if let Some(tray) = ctx.module.find_tray(id) {
                        tray.update_title(title.clone());
                    }
                }
                Event::IconUpdate(tray_icon) => {
                    let size = ctx.module.icon_size;
                    if let Some(tray) = ctx.module.find_tray(id) {
                        tray.update_icon(TrayModule::parse_tray_icon(tray_icon, size));
                    }
                }
                Event::MenuNew(tray_menu) => {}
                _ => {} // Event::MenuNew(tray_menu) => todo!(),
            }
        }
    )));

    ctx.borrow_mut().backend_cb_id = backend_cb_id;

    ctx
}
