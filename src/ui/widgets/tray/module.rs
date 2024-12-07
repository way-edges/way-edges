use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use cairo::ImageSurface;
use system_tray::item::StatusNotifierItem;

use crate::{
    config::widgets::wrapbox::Align,
    plug::tray::{
        icon::{parse_icon_given_data, parse_icon_given_name, parse_icon_given_pixmaps},
        tray_update_item_theme_search_path,
    },
};

use crate::ui::widgets::wrapbox::{
    display::grid::{DisplayWidget, GridBox},
    expose::BoxRedrawFunc,
};

use super::layout::TrayLayout;

#[derive(Default)]
pub struct MenuState {
    pub open_state: HashSet<i32>,
    pub hover_state: i32,
}
impl MenuState {
    pub fn is_open(&self, id: i32) -> bool {
        self.open_state.contains(&id)
    }
    pub fn is_hover(&self, id: i32) -> bool {
        self.hover_state == id
    }

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
            fn check_open_state(&mut self, menu: &MenuItem) {
                if !self.need_check_open_state {
                    return;
                }

                if menu.submenu.is_some() {
                    self.new_open_state.as_mut().unwrap().insert(menu.id);
                }
            }
            fn post_check_open_state(&mut self) {
                if let Some(new_open_state) = self.new_open_state.take() {
                    self.state.open_state = new_open_state;
                }
            }
            fn check_hover_state(&mut self, menu: &MenuItem) {
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
            fn iter_menus(&mut self, vec: &[MenuItem]) {
                vec.iter().for_each(|menu| {
                    self.check_open_state(menu);
                    self.check_hover_state(menu);
                    if let Some(submenu) = &menu.submenu {
                        self.iter_menus(submenu);
                    }
                });
            }
        }
    }
}

pub struct RootMenu {
    pub id: i32,
    pub submenus: Vec<MenuItem>,
}
impl RootMenu {
    pub fn from_tray_menu(tray_menu: &system_tray::menu::TrayMenu, icon_size: i32) -> Self {
        Self {
            id: tray_menu.id as i32,
            submenus: tray_menu.submenus.vec_into_menu(icon_size),
        }
    }
}
trait VecTrayMenuIntoVecLocalMenuItem {
    fn vec_into_menu(&self, icon_size: i32) -> Vec<MenuItem>;
}
impl VecTrayMenuIntoVecLocalMenuItem for Vec<system_tray::menu::MenuItem> {
    fn vec_into_menu(&self, icon_size: i32) -> Vec<MenuItem> {
        self.iter()
            .map(|item| MenuItem::from_menu_item(item, icon_size))
            .collect()
    }
}

pub struct MenuItem {
    pub id: i32,
    pub label: Option<String>,
    pub enabled: bool,
    pub icon: Option<ImageSurface>,
    pub menu_type: MenuType,

    pub submenu: Option<Vec<MenuItem>>,
}

impl MenuItem {
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
                    system_tray::menu::ToggleType::CannotBeToggled => MenuType::Normal,
                }
            }
        };

        let submenu = if !value.submenu.is_empty() {
            Some(
                value
                    .submenu
                    .iter()
                    .map(|item| MenuItem::from_menu_item(item, icon_size))
                    .collect(),
            )
        } else {
            None
        };

        Self {
            id,
            label,
            enabled,
            icon,
            menu_type,
            submenu,
        }
    }
}

pub enum MenuType {
    Radio(bool),
    Check(bool),
    // should the menu wtih submenus have toggle states?
    Separator,
    Normal,
}

pub struct Tray {
    pub tray_id: TrayID,
    pub id: String,
    pub title: Option<String>,
    pub icon: ImageSurface,
    pub menu_path: Option<String>,
    pub menu: Option<(RootMenu, MenuState)>,

    pub is_open: bool,

    pub updated: bool,
    pub content: ImageSurface,
    pub layout: TrayLayout,
}

impl Tray {
    pub fn update_title(&mut self, title: Option<String>) {
        if title != self.title {
            self.title = title;
            self.set_updated();
        }
    }
    pub fn update_icon(&mut self, icon: ImageSurface) {
        self.icon = icon;
        self.set_updated();
    }
    pub fn update_menu(&mut self, new: RootMenu) {
        if let Some((old, state)) = &mut self.menu {
            state.filter_state_with_new_menu(&new);
            *old = new;
        } else {
            self.menu = Some((new, MenuState::default()));
        }
        self.set_updated();
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
    fn draw(&mut self) {
        TrayLayout::draw_and_create(self);
    }

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

            is_open: false,

            layout: TrayLayout::default(),
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

pub type TrayID = Rc<String>;

pub struct TrayModule {
    // id
    pub grid: GridBox<TrayID>,
    pub id_tray_map: HashMap<TrayID, Tray>,
    pub redraw_signal: BoxRedrawFunc,

    pub icon_size: i32,
}
impl TrayModule {
    pub fn draw_content(&mut self) -> ImageSurface {
        self.grid.draw(
            |id| self.id_tray_map.get(id).unwrap().get_size(),
            |id| self.id_tray_map.get(id).unwrap().content(),
        )
    }
    pub fn new(redraw_signal: BoxRedrawFunc) -> Self {
        let grid = GridBox::new(0., Align::CenterCenter);
        Self {
            grid,
            id_tray_map: HashMap::new(),
            redraw_signal,

            icon_size: 16,
        }
    }
    pub fn add_tray(&mut self, id: String, tray_item: &StatusNotifierItem) {
        let id = Rc::new(id);
        let tray = Tray::from_notify_item(id.clone(), tray_item, self.icon_size);

        self.grid.add(id.clone());
        self.id_tray_map.insert(id, tray);

        (self.redraw_signal)()
    }
    pub fn remove_tray(&mut self, id: &String) {
        self.grid.rm(id);
        self.id_tray_map.remove(id);

        (self.redraw_signal)()
    }

    pub fn find_tray(&mut self, id: &String) -> Option<&mut Tray> {
        self.id_tray_map.get_mut(id)
    }
}
