mod draw;
mod item;
mod layout;
mod module;

use std::rc::Rc;

use cairo::ImageSurface;

use backend::tray::{
    icon::parse_icon_given_name, init_tray_client, register_tray, unregister_tray, Event,
};
use config::widgets::wrapbox::tray::TrayConfig;
use glib::clone::{Downgrade, Upgrade};
use item::RootMenu;
use module::{new_tray_module, TrayModuleRc};
use util::Or;

use crate::{
    mouse_state::MouseEvent,
    widgets::wrapbox::{box_traits::BoxedWidget, BoxTemporaryCtx},
};

#[derive(Debug)]
pub struct TrayCtx {
    module: TrayModuleRc,
    backend_cb_id: i32,
}
impl Drop for TrayCtx {
    fn drop(&mut self) {
        unregister_tray(self.backend_cb_id);
    }
}

impl BoxedWidget for TrayCtx {
    fn content(&mut self) -> cairo::ImageSurface {
        self.module.borrow_mut().draw_content()
    }

    fn on_mouse_event(&mut self, e: MouseEvent) -> bool {
        let mut redraw = Or(false);

        match e {
            MouseEvent::Release(pos, key) => {
                let mut m = self.module.borrow_mut();

                if let Some((tray, pos)) = m.match_tray_id_from_pos(pos) {
                    redraw.or(m.replace_current_tray(tray.clone()));
                    redraw.or(tray
                        .borrow_mut()
                        .on_mouse_event(MouseEvent::Release(pos, key)));
                }
            }
            MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                let mut m = self.module.borrow_mut();

                if let Some((tray, pos)) = m.match_tray_id_from_pos(pos) {
                    redraw.or(m.replace_current_tray(tray.clone()));
                    redraw.or(tray.borrow_mut().on_mouse_event(MouseEvent::Motion(pos)));
                }
            }
            MouseEvent::Leave => {
                redraw.or(self.module.borrow_mut().leave_last_tray());
            }
            _ => {}
        }

        redraw.res()
    }
}

pub fn init_widget(box_temp_ctx: &mut BoxTemporaryCtx, config: TrayConfig) -> TrayCtx {
    init_tray_client();

    let module = new_tray_module(config).make_rc();

    let module_weak = module.downgrade();
    let s = box_temp_ctx.make_redraw_channel(move |_, msg: Rc<(String, Event)>| {
        let Some(module) = module_weak.upgrade() else {
            return;
        };
        let id = &msg.0;
        let e = &msg.1;

        let mut m = module.borrow_mut();
        match e {
            Event::ItemNew(tray_item) => {
                m.add_tray(id.clone(), tray_item.as_ref());
            }
            Event::ItemRemove => {
                m.remove_tray(id);
            }
            Event::TitleUpdate(title) => {
                if let Some(tray) = m.find_tray(id) {
                    tray.borrow_mut().update_title(title.clone());
                }
            }
            Event::IconUpdate(tray_icon) => {
                if let Some(tray) = m.find_tray(id) {
                    let size = m.config.icon_size;
                    let theme = m.config.icon_theme.as_deref();
                    let surf = parse_icon_given_name(tray_icon, size, theme).unwrap_or(
                        ImageSurface::create(cairo::Format::ARgb32, size, size).unwrap(),
                    );
                    tray.borrow_mut().update_icon(surf);
                }
            }
            Event::MenuNew(tray_menu) => {
                if let Some(tray) = m.find_tray(id) {
                    let size = m.config.icon_size;
                    let theme = m.config.icon_theme.as_deref();
                    let root_menu = RootMenu::from_tray_menu(tray_menu, size, theme);
                    tray.borrow_mut().update_menu(root_menu);
                }
            }
        }
    });

    let backend_cb_id = register_tray(s);

    TrayCtx {
        module,
        backend_cb_id,
    }
}
