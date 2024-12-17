use std::{collections::HashMap, rc::Rc};

use cairo::ImageSurface;
use gtk::gdk::{BUTTON_PRIMARY, BUTTON_SECONDARY};
use system_tray::{client::ActivateRequest, item::StatusNotifierItem};

use crate::get_main_runtime_handle;
use backend::tray::{
    icon::{parse_icon_given_data, parse_icon_given_name, parse_icon_given_pixmaps},
    tray_request_event, tray_update_item_theme_search_path,
};
use config::widgets::wrapbox::tray::TrayConfig;
use util::notify_send;

use crate::ui::widgets::wrapbox::{
    display::grid::{DisplayWidget, GridBox},
    expose::BoxRedrawFunc,
};

use super::layout::TrayLayout;

#[derive(Default)]
pub struct MenuState {
    // pub open_state: HashSet<i32>,
    pub open_state: Box<[i32]>,
    pub hover_state: i32,
}
impl MenuState {
    pub fn is_open(&self, id: i32) -> bool {
        self.open_state.contains(&id)
    }
    pub fn is_hover(&self, id: i32) -> bool {
        self.hover_state == id
    }
    fn set_hovering(&mut self, id: i32) -> bool {
        if self.hover_state != id {
            self.hover_state = id;
            true
        } else {
            false
        }
    }
    fn set_open_id(&mut self, mut id_chain: Vec<i32>) {
        let clicked_one = id_chain.last().unwrap();
        if self.open_state.contains(clicked_one) {
            id_chain.pop();
        }
        self.open_state = id_chain.into_boxed_slice();
    }

    fn filter_state_with_new_menu(&mut self, menu: &RootMenu) {
        Checker::run(self, menu);

        struct Checker<'a> {
            state: &'a mut MenuState,

            new_open_state: Option<Vec<i32>>,
            found_hover: Option<bool>,
        }
        impl<'a> Checker<'a> {
            fn run(state: &'a mut MenuState, menu: &RootMenu) {
                let need_check_open_state = !state.open_state.is_empty();
                let need_check_hover = state.hover_state != -1;

                let mut checker = Checker {
                    state,

                    new_open_state: if need_check_open_state {
                        Some(vec![])
                    } else {
                        None
                    },
                    found_hover: if need_check_hover { Some(false) } else { None },
                };

                checker.iter_menus(&menu.submenus, 0);
                checker.post_check_open_state();
                checker.post_check_hover_state();
            }
            fn check_open_state(&mut self, menu: &MenuItem, level: usize) {
                if let Some(new_open_state) = self.new_open_state.as_mut() {
                    if level < self.state.open_state.len()
                        && self.state.open_state[level] == menu.id
                        && menu.submenu.is_some()
                    {
                        new_open_state.push(menu.id);
                    }
                }
            }
            fn post_check_open_state(&mut self) {
                if let Some(new_open_state) = self.new_open_state.take() {
                    self.state.open_state = new_open_state.into_boxed_slice();
                }
            }
            fn check_hover_state(&mut self, menu: &MenuItem, _: usize) {
                if let Some(found_hover) = self.found_hover.as_mut() {
                    if menu.id == self.state.hover_state {
                        *found_hover = true;
                    }
                }
            }
            fn post_check_hover_state(&mut self) {
                if let Some(found_hover) = self.found_hover.take() {
                    if !found_hover {
                        self.state.hover_state = -1;
                    }
                }
            }
            fn iter_menus(&mut self, vec: &[MenuItem], level: usize) {
                vec.iter().for_each(|menu| {
                    self.check_open_state(menu, level);
                    self.check_hover_state(menu, level);
                    if let Some(submenu) = &menu.submenu {
                        self.iter_menus(submenu, level + 1);
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

    tary_module: *const TrayModule,
    // redraw_signal: BoxRedrawFunc,
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

    fn send_request(req: ActivateRequest) {
        get_main_runtime_handle().spawn(async move {
            if let Err(e) = tray_request_event(req).await {
                let msg = format!("error requesting tray activation: {e}");
                log::error!("{msg}");
                notify_send("Tray activation", &msg, true);
            }
        });
    }

    pub fn tray_clicked_req(&self) {
        let address = String::clone(&self.tray_id);
        Self::send_request(ActivateRequest::Default {
            address,
            x: 0,
            y: 0,
        });
    }

    pub fn menu_item_clicked_req(&self, submenu_id: i32) {
        if let Some(menu_path) = self.menu_path.as_ref() {
            let address = String::clone(&self.tray_id);
            let menu_path = menu_path.clone();

            Self::send_request(ActivateRequest::MenuItem {
                address,
                menu_path,
                submenu_id,
            });
        }
    }

    pub fn get_module(&self) -> &TrayModule {
        unsafe { self.tary_module.as_ref() }.unwrap()
    }
}
impl Tray {
    fn get_menu_state(&mut self) -> Option<&mut (RootMenu, MenuState)> {
        self.menu.as_mut()
    }
    fn set_updated(&mut self) {
        self.updated = true;
        (self.get_module().redraw_signal)();
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

    fn from_notify_item(
        module: &TrayModule,
        tray_id: TrayID,
        value: &StatusNotifierItem,
        icon_size: i32,
    ) -> Self {
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

            tary_module: module as *const TrayModule,
        }
    }
}

impl DisplayWidget for Tray {
    fn get_size(&self) -> (f64, f64) {
        let ptr = self as *const Tray as *mut Tray;
        unsafe { ptr.as_mut() }.unwrap().redraw_if_updated();

        (self.content.width() as f64, self.content.height() as f64)
    }

    fn content(&self) -> ImageSurface {
        let ptr = self as *const Tray as *mut Tray;
        unsafe { ptr.as_mut() }.unwrap().redraw_if_updated();

        self.content.clone()
    }

    fn on_mouse_event(&mut self, e: crate::ui::draws::mouse_state::MouseEvent) {
        use super::layout::HoveringItem;
        use crate::ui::draws::mouse_state::MouseEvent;
        match e {
            MouseEvent::Release(pos, key) => {
                let Some(hovering) = self.layout.get_hovering(pos) else {
                    return;
                };

                if key == BUTTON_SECONDARY {
                    // toggle state
                    match hovering {
                        HoveringItem::TrayIcon => {
                            self.is_open = !self.is_open;
                            self.set_updated();
                        }
                        HoveringItem::MenuItem(id) => {
                            // find id chain
                            fn find_id_chain(
                                menu: &[MenuItem],
                                id: i32,
                                chain: &mut Vec<i32>,
                            ) -> bool {
                                for i in menu.iter() {
                                    // only happens for a parent menu
                                    if let Some(submenu) = &i.submenu {
                                        if i.id == id || find_id_chain(submenu, id, chain) {
                                            chain.push(i.id);
                                            return true;
                                        }
                                    }
                                }
                                false
                            }

                            let mut id_chain = vec![];
                            let (root, state) = self.get_menu_state().unwrap();
                            find_id_chain(&root.submenus, id, &mut id_chain);

                            if !id_chain.is_empty() {
                                id_chain.reverse();
                                state.set_open_id(id_chain);
                                self.set_updated();
                            }
                        }
                    }
                } else if key == BUTTON_PRIMARY {
                    match hovering {
                        HoveringItem::TrayIcon => self.tray_clicked_req(),
                        HoveringItem::MenuItem(id) => self.menu_item_clicked_req(id),
                    }
                }
            }
            MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                let mut hover_id = -1;

                if let Some(HoveringItem::MenuItem(id)) = self.layout.get_hovering(pos) {
                    hover_id = id;
                }

                if self.get_menu_state().unwrap().1.set_hovering(hover_id) {
                    self.set_updated();
                }
            }
            MouseEvent::Leave => {
                if let Some((_, state)) = self.get_menu_state() {
                    if state.set_hovering(-1) {
                        self.set_updated();
                    }
                }
            }
            // ignore press
            _ => {}
        }
    }
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

pub struct TrayModuleState {
    current_mouse_in: Option<TrayID>,
}
impl TrayModuleState {
    fn new() -> Self {
        Self {
            current_mouse_in: None,
        }
    }
    pub fn set_current_tary(&mut self, id: TrayID) -> Option<TrayID> {
        let Some(old) = self.current_mouse_in.as_mut() else {
            return self.current_mouse_in.replace(id);
        };

        if old == &id {
            return None;
        }

        // drop the mut reference
        // though this can be not written explicitly
        #[allow(dropping_references)]
        drop(old);

        self.current_mouse_in.replace(id)
    }
}

pub type TrayID = Rc<String>;

pub struct TrayModule {
    // id
    pub grid: GridBox<TrayID>,
    pub id_tray_map: HashMap<TrayID, Tray>,
    pub redraw_signal: BoxRedrawFunc,

    pub config: TrayConfig,

    pub module_state: TrayModuleState,
}
impl TrayModule {
    pub fn draw_content(&mut self) -> ImageSurface {
        self.grid.draw(
            |id| self.id_tray_map.get(id).unwrap().get_size(),
            |id| self.id_tray_map.get(id).unwrap().content(),
        )
    }
    pub fn new(redraw_signal: BoxRedrawFunc, config: TrayConfig) -> Self {
        let grid = GridBox::new(config.tray_gap as f64, config.grid_align);
        Self {
            grid,
            id_tray_map: HashMap::new(),
            redraw_signal,

            config,

            module_state: TrayModuleState::new(),
        }
    }
    pub fn add_tray(&mut self, id: String, tray_item: &StatusNotifierItem) {
        let id = Rc::new(id);

        if self.id_tray_map.contains_key(&id) {
            return;
        }

        let tray = Tray::from_notify_item(self, id.clone(), tray_item, self.config.icon_size);

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

    pub fn match_tray_id_from_pos(&self, pos: (f64, f64)) -> Option<(&TrayID, (f64, f64))> {
        self.grid.position_map.as_ref().unwrap().match_item(pos)
    }

    pub fn leave_last_tray(&mut self) {
        if let Some(f) = self
            .module_state
            .current_mouse_in
            .take()
            .and_then(|last_id| self.id_tray_map.get_mut(&last_id))
        {
            f.on_mouse_event(crate::ui::draws::mouse_state::MouseEvent::Leave);
        }
    }

    pub fn replace_current_tray(&mut self, id: TrayID) {
        if let Some(f) = self
            .module_state
            .set_current_tary(id)
            .and_then(|last_id| self.id_tray_map.get_mut(&last_id))
        {
            f.on_mouse_event(crate::ui::draws::mouse_state::MouseEvent::Leave);
        }
    }
}
