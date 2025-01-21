use std::rc::Rc;

use cairo::ImageSurface;
use smithay_client_toolkit::seat::pointer::{BTN_LEFT, BTN_RIGHT};
use system_tray::{event::ActivateRequest, item::StatusNotifierItem};

use backend::tray::{
    icon::{parse_icon_given_data, parse_icon_given_name, parse_icon_given_pixmaps},
    tray_request_event, tray_update_item_theme_search_path,
};
use config::widgets::wrapbox::tray::TrayConfig;
use way_edges_derive::wrap_rc;

use crate::{
    buffer::Buffer, mouse_state::MouseEvent, widgets::wrapbox::grid::item::GridItemContent,
};

use super::{layout::TrayLayout, module::TrayModule};

pub type TrayID = Rc<String>;

#[derive(Default, Debug)]
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

#[derive(Debug)]
pub struct RootMenu {
    #[allow(dead_code)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum MenuType {
    Radio(bool),
    Check(bool),
    // should the menu wtih submenus have toggle states?
    Separator,
    Normal,
}

#[wrap_rc(rc = "pub", normal = "pub")]
#[derive(Debug)]
pub struct Tray {
    pub tray_id: TrayID,
    pub id: String,
    pub title: Option<String>,
    pub icon: ImageSurface,
    pub menu_path: Option<String>,
    pub menu: Option<(RootMenu, MenuState)>,

    pub is_open: bool,

    pub updated: bool,
    pub layout: TrayLayout,
    pub buffer: Buffer,

    pub config: Rc<TrayConfig>,
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
        tray_request_event(req)
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
}
impl Tray {
    fn get_menu_state(&mut self) -> Option<&mut (RootMenu, MenuState)> {
        self.menu.as_mut()
    }
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
        let (buf, ly) = TrayLayout::draw_and_create(self);
        self.buffer.update_buffer(buf);
        self.layout = ly
    }
}

impl GridItemContent for TrayRc {
    fn draw(&mut self) -> ImageSurface {
        let mut s = self.borrow_mut();
        s.redraw_if_updated();
        s.buffer.get_buffer()
    }
}
impl Tray {
    pub fn on_mouse_event(&mut self, e: MouseEvent) -> bool {
        use super::layout::HoveringItem;
        let mut redraw = false;

        match e {
            MouseEvent::Release(pos, key) => {
                let Some(hovering) = self.layout.get_hovering(pos) else {
                    return false;
                };

                if key == BTN_RIGHT {
                    // toggle state
                    match hovering {
                        HoveringItem::TrayIcon => {
                            self.is_open = !self.is_open;
                            self.set_updated();
                            redraw = true
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
                                redraw = true
                            }
                        }
                    }
                } else if key == BTN_LEFT {
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
                    redraw = true
                }
            }
            MouseEvent::Leave => {
                if let Some((_, state)) = self.get_menu_state() {
                    if state.set_hovering(-1) {
                        self.set_updated();
                        redraw = true
                    }
                }
            }
            // ignore press
            _ => {}
        }
        redraw
    }
}

impl Eq for TrayRc {}
impl PartialEq for TrayRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

pub fn create_tray_item(
    module: &TrayModule,
    tray_id: TrayID,
    value: &StatusNotifierItem,
    icon_size: i32,
) -> TrayRc {
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

    Tray {
        tray_id,
        id,
        title,
        icon,
        menu_path,
        menu: None,
        updated: true,

        is_open: false,
        layout: TrayLayout::default(),
        buffer: Buffer::default(),
        config: module.config.clone(),
    }
    .make_rc()
}
