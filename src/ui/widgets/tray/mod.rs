use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use cairo::ImageSurface;
use system_tray::item::StatusNotifierItem;

use crate::{
    config::widgets::wrapbox::Align,
    plug::tray::{
        icon::{parse_icon_given_data, parse_icon_given_name, parse_icon_given_pixmaps},
        register_tray, tray_update_item_theme_search_path, unregister_tray,
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
    fn filter_state_with_new_menu(&mut self, menu: &RootMenu) {
        Checker::run(self, menu);

        struct Checker<'a> {
            need_check_open_state: bool,
            need_check_hover: bool,
            state: &'a mut MenuState,

            new_open_state: Option<HashSet<i32>>,
            found_hover: Option<bool>,
        }
        impl<'a> Checker<'a> {
            fn run(state: &'a mut MenuState, menu: &RootMenu) {
                let need_check_open_state = !state.open_state.is_empty();
                let need_check_hover = state.hover_state != -1;

                let mut checker = Checker {
                    need_check_open_state,
                    need_check_hover,
                    state,

                    new_open_state: if need_check_open_state {
                        Some(HashSet::new())
                    } else {
                        None
                    },
                    found_hover: if need_check_hover { Some(false) } else { None },
                };

                checker.iter_menus(&menu.submenus);
                checker.post_check_open_state();
                checker.post_check_hover_state();
            }
            fn check_open_state(&mut self, menu: &Menu) {
                if !self.need_check_open_state {
                    return;
                }

                if let MenuType::Parent(_) = &menu.menu_type {
                    self.new_open_state.as_mut().unwrap().insert(menu.id);
                }
            }
            fn post_check_open_state(&mut self) {
                if let Some(new_open_state) = self.new_open_state.take() {
                    self.state.open_state = new_open_state;
                }
            }
            fn check_hover_state(&mut self, menu: &Menu) {
                if !self.need_check_hover {
                    return;
                }
                if menu.id == self.state.hover_state {
                    self.found_hover.replace(true);
                }
            }
            fn post_check_hover_state(&mut self) {
                if let Some(found_hover) = self.found_hover.take() {
                    if !found_hover {
                        self.state.hover_state = -1;
                    }
                }
            }
            fn iter_menus(&mut self, vec: &[Menu]) {
                vec.iter().for_each(|menu| {
                    self.check_open_state(menu);
                    self.check_hover_state(menu);
                    if let MenuType::Parent(submenus) = &menu.menu_type {
                        self.iter_menus(submenus);
                    }
                });
            }
        }
    }
}

struct RootMenu {
    id: i32,
    submenus: Vec<Menu>,
}
impl RootMenu {
    fn from_tray_menu(tray_menu: &system_tray::menu::TrayMenu, icon_size: i32) -> Self {
        Self {
            id: tray_menu.id as i32,
            submenus: tray_menu.submenus.vec_into_menu(icon_size),
        }
    }
}
trait VecTrayMenuIntoVecMenu {
    fn vec_into_menu(&self, icon_size: i32) -> Vec<Menu>;
}
impl VecTrayMenuIntoVecMenu for Vec<system_tray::menu::MenuItem> {
    fn vec_into_menu(&self, icon_size: i32) -> Vec<Menu> {
        self.iter()
            .map(|item| Menu::from_menu_item(item, icon_size))
            .collect()
    }
}

struct Menu {
    id: i32,
    label: Option<String>,
    enabled: bool,
    icon: Option<ImageSurface>,
    menu_type: MenuType,
}

impl Menu {
    fn from_menu_item(value: &system_tray::menu::MenuItem, icon_size: i32) -> Self {
        let id = value.id;
        let label = value.label.clone();
        let enabled = value.enabled;

        let icon = value
            .icon_name
            .clone()
            .filter(|name| !name.is_empty())
            .and_then(|name| parse_icon_given_name(&name, icon_size))
            .or(value.icon_data.clone().and_then(parse_icon_given_data));

        let menu_type = match value.menu_type {
            system_tray::menu::MenuType::Separator => MenuType::Separator,
            system_tray::menu::MenuType::Standard => {
                match value.toggle_type {
                    system_tray::menu::ToggleType::Checkmark => {
                        MenuType::Check(match value.toggle_state {
                            system_tray::menu::ToggleState::On => true,
                            system_tray::menu::ToggleState::Off => false,
                            system_tray::menu::ToggleState::Indeterminate => {
                                log::error!("THIS SHOULD NOT HAPPEN. menu item has toggle but not toggle state");
                                // ???
                                false
                            }
                        })
                    }
                    system_tray::menu::ToggleType::Radio => {
                        MenuType::Radio(match value.toggle_state {
                            system_tray::menu::ToggleState::On => true,
                            system_tray::menu::ToggleState::Off => false,
                            system_tray::menu::ToggleState::Indeterminate => {
                                log::error!("THIS SHOULD NOT HAPPEN. menu item has toggle but not toggle state");
                                // ???
                                false
                            }
                        })
                    }
                    system_tray::menu::ToggleType::CannotBeToggled => {
                        if !value.submenu.is_empty() {
                            MenuType::Parent(
                                value
                                    .submenu
                                    .iter()
                                    .map(|item| Menu::from_menu_item(item, icon_size))
                                    .collect(),
                            )
                        } else {
                            MenuType::Normal
                        }
                    }
                }
            }
        };

        Self {
            id,
            label,
            enabled,
            icon,
            menu_type,
        }
    }
}

enum MenuType {
    Radio(bool),
    Check(bool),
    // should the menu wtih submenus have toggle states?
    Parent(Vec<Menu>),
    Separator,
    Normal,
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

    fn from_notify_item(tray_id: TrayID, value: &StatusNotifierItem, icon_size: i32) -> Self {
        let id = value.id.clone();
        let title = value.title.clone();

        if let Some(theme) = value.icon_theme_path.clone() {
            tray_update_item_theme_search_path(theme);
        }

        // NOTE: THIS LOOK RIDICULOUS I KNOW, ANY BETTER IDEA? I'M FRUSTRATED.
        let icon = value
            .icon_name
            .clone()
            .filter(|icon_name| !icon_name.is_empty())
            .and_then(|name| parse_icon_given_name(&name, icon_size))
            .or(value
                .icon_pixmap
                .as_ref()
                .and_then(|icon_pix_map| parse_icon_given_pixmaps(icon_pix_map, icon_size)))
            .unwrap_or(ImageSurface::create(cairo::Format::ARgb32, icon_size, icon_size).unwrap());

        let menu_path = value.menu.clone();

        Self {
            tray_id,
            id,
            title,
            icon,
            menu_path,
            menu: None,
            updated: true,
            content: ImageSurface::create(cairo::Format::ARgb32, 0, 0).unwrap(),
        }
    }
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
    fn add_tray(&mut self, id: String, tray_item: &StatusNotifierItem) {
        let id = Rc::new(id);
        let tray = Tray::from_notify_item(id.clone(), tray_item, self.icon_size);

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
                        let surf = parse_icon_given_name(tray_icon, size).unwrap_or(
                            ImageSurface::create(cairo::Format::ARgb32, size, size).unwrap(),
                        );
                        tray.update_icon(surf);
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
