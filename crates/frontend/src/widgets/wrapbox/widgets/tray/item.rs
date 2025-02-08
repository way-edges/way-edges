use std::sync::Arc;

use cairo::ImageSurface;
use smithay_client_toolkit::seat::pointer::{BTN_LEFT, BTN_RIGHT};
use system_tray::client::ActivateRequest;

use backend::tray::{
    item::{MenuItem, RootMenu, Tray},
    tray_about_to_show_menuitem, tray_active_request,
};
use config::widgets::wrapbox::tray::TrayConfig;

use crate::{buffer::Buffer, mouse_state::MouseEvent};

use super::layout::TrayLayout;

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
struct MenuIDTreeNode {
    id: i32,
    sub: Option<Vec<MenuIDTreeNode>>,
}
impl MenuIDTreeNode {
    fn from_root_menu(root: &RootMenu) -> Self {
        Self {
            id: root.id,
            sub: Some(Self::from_menu(&root.submenus)),
        }
    }
    fn from_menu(menu: &[MenuItem]) -> Vec<Self> {
        menu.iter()
            .map(|m| Self {
                id: m.id,
                sub: m.submenu.as_deref().map(Self::from_menu),
            })
            .collect()
    }
}

#[derive(Debug)]
struct TrayCacheData {
    dest: Arc<String>,
    menu_path: Option<String>,
    menu_id_tree: Option<MenuIDTreeNode>,
}

#[derive(Debug)]
pub struct TrayState {
    tray_cache_data: TrayCacheData,
    pub menu_state: MenuState,
    pub is_open: bool,
    pub updated: bool,

    layout: TrayLayout,
    buffer: Buffer,
}

// update
impl TrayState {
    pub fn new(dest: Arc<String>, tray: &Tray) -> Self {
        Self {
            tray_cache_data: TrayCacheData {
                dest,
                menu_path: tray.menu_path.clone(),
                menu_id_tree: tray.menu.as_ref().map(MenuIDTreeNode::from_root_menu),
            },
            menu_state: MenuState::default(),

            is_open: false,
            updated: true,
            layout: TrayLayout::default(),
            buffer: Buffer::default(),
        }
    }
    pub fn update_tray(&mut self, tray: &Tray) {
        self.updated = true;
        tray.menu.as_ref().inspect(|f| {
            self.tray_cache_data.menu_id_tree = Some(MenuIDTreeNode::from_root_menu(f));
            self.menu_state.filter_state_with_new_menu(f);
        });
    }
}

// proxy request
impl TrayState {
    fn send_active_request(req: ActivateRequest) {
        tray_active_request(req)
    }
    fn tray_clicked_req(&self) {
        let address = String::clone(&self.tray_cache_data.dest);
        Self::send_active_request(ActivateRequest::Default {
            address,
            x: 0,
            y: 0,
        });
    }
    fn menu_item_clicked_req(&self, submenu_id: i32) {
        if let Some(menu_path) = self.tray_cache_data.menu_path.as_ref() {
            let address = String::clone(&self.tray_cache_data.dest);
            let menu_path = menu_path.clone();

            Self::send_active_request(ActivateRequest::MenuItem {
                address,
                menu_path,
                submenu_id,
            });
        }
    }
    fn menuitem_about_to_show(&self, menuitem_id: i32) {
        if let Some(path) = self.tray_cache_data.menu_path.as_ref() {
            tray_about_to_show_menuitem(
                self.tray_cache_data.dest.to_string(),
                path.to_string(),
                menuitem_id,
            );
        }
    }
}

// content
impl TrayState {
    // recalculate id chain
    // for menu:
    // a:
    //  - b
    //  - c:
    //    - e
    //  - d
    //
    //  pressing e will produce id chain: a,c,e
    //  but the result is e,c,a so we need to reverse it after this
    fn recalculate_open_id_chain(&mut self, id: i32) -> bool {
        fn calculate_id_chain(menu: &[MenuIDTreeNode], id: i32, chain: &mut Vec<i32>) -> bool {
            for i in menu.iter() {
                // only happens for a parent menu
                if let Some(submenu) = &i.sub {
                    if i.id == id || calculate_id_chain(submenu, id, chain) {
                        chain.push(i.id);
                        return true;
                    }
                }
            }
            false
        }

        let mut id_chain = vec![];
        calculate_id_chain(
            self.tray_cache_data
                .menu_id_tree
                .as_ref()
                .unwrap()
                .sub
                .as_ref()
                .unwrap(),
            id,
            &mut id_chain,
        );
        id_chain.reverse();

        if !id_chain.is_empty() {
            id_chain.reverse();
            self.menu_state.set_open_id(id_chain);
            true
        } else {
            false
        }
    }
    fn set_updated(&mut self) {
        self.updated = true;
    }
    fn redraw_if_updated(&mut self, tray: &Tray, conf: &TrayConfig) {
        if self.updated {
            self.updated = false;

            let (buf, ly) = TrayLayout::draw_and_create(self, tray, conf);
            self.buffer.update_buffer(buf);
            self.layout = ly;
        }
    }
    pub fn draw(&mut self, tray: &Tray, conf: &TrayConfig) -> ImageSurface {
        self.redraw_if_updated(tray, conf);
        self.buffer.get_buffer()
    }
}

impl TrayState {
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
                    let menuitem_id = match hovering {
                        HoveringItem::TrayIcon => {
                            self.is_open = !self.is_open;
                            Some(0)
                        }
                        HoveringItem::MenuItem(id) => {
                            if self.recalculate_open_id_chain(id) {
                                Some(id)
                            } else {
                                None
                            }
                        }
                    };

                    // send about to show request or else some menu won't appear
                    if let Some(id) = menuitem_id {
                        self.set_updated();
                        redraw = true;
                        self.menuitem_about_to_show(id);
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

                if self.menu_state.set_hovering(hover_id) {
                    self.set_updated();
                    redraw = true
                }
            }
            MouseEvent::Leave => {
                if self.menu_state.set_hovering(-1) {
                    self.set_updated();
                    redraw = true
                }
            }
            // ignore press
            _ => {}
        }
        redraw
    }
}
