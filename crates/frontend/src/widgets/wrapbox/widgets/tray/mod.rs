mod draw;
mod item;
mod layout;
mod module;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use backend::tray::{init_tray_client, register_tray, TrayBackendHandle, TrayMsg};
use config::widgets::wrapbox::tray::TrayConfig;
use module::{new_tray_module, TrayModule};
use util::Or;

use crate::{
    mouse_state::MouseEvent,
    widgets::wrapbox::{box_traits::BoxedWidget, BoxTemporaryCtx},
};

#[derive(Debug)]
pub struct TrayCtx {
    module: TrayModule,
    backend_handle: TrayBackendHandle,
}

impl TrayCtx {
    fn content(&mut self) -> cairo::ImageSurface {
        self.module
            .draw_content(&self.backend_handle.get_tray_map().lock().unwrap())
    }

    fn on_mouse_event(&mut self, e: MouseEvent) -> bool {
        let mut redraw = Or(false);

        match e {
            MouseEvent::Release(pos, key) => {
                if let Some((dest, tray, pos)) = self.module.match_tray_id_from_pos(pos) {
                    redraw.or(tray.on_mouse_event(MouseEvent::Release(pos, key)));
                    redraw.or(self.module.replace_current_tray(dest));
                }
            }
            MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => {
                if let Some((dest, tray, pos)) = self.module.match_tray_id_from_pos(pos) {
                    redraw.or(tray.on_mouse_event(MouseEvent::Motion(pos)));
                    redraw.or(self.module.replace_current_tray(dest));
                }
            }
            MouseEvent::Leave => {
                redraw.or(self.module.leave_last_tray());
            }
            _ => {}
        }

        redraw.res()
    }
}

#[derive(Debug)]
pub struct TrayCtxRc(Rc<RefCell<TrayCtx>>);
impl BoxedWidget for TrayCtxRc {
    fn content(&mut self) -> cairo::ImageSurface {
        self.0.borrow_mut().content()
    }

    fn on_mouse_event(&mut self, e: MouseEvent) -> bool {
        self.0.borrow_mut().on_mouse_event(e)
    }
}

pub fn init_widget(box_temp_ctx: &mut BoxTemporaryCtx, config: TrayConfig) -> TrayCtxRc {
    init_tray_client();

    let rc = Rc::new_cyclic(|weak: &Weak<RefCell<TrayCtx>>| {
        let weak = weak.clone();
        let s = box_temp_ctx.make_redraw_channel(move |_, dest: TrayMsg| {
            let Some(module) = weak.upgrade() else {
                return;
            };

            let mut m = module.borrow_mut();

            use backend::tray::TrayEventSignal::*;
            match dest {
                Add(dest) => {
                    let tray_map_ptr = m.backend_handle.get_tray_map();
                    let tray_map = tray_map_ptr.lock().unwrap();
                    let tray = tray_map.get_tray(&dest).unwrap();
                    m.module.add_tray(dest, tray);
                }
                Rm(dest) => {
                    m.module.remove_tray(&dest);
                }
                Update(dest) => {
                    let tray_map_ptr = m.backend_handle.get_tray_map();
                    let tray_map = tray_map_ptr.lock().unwrap();
                    let tray = tray_map.get_tray(&dest).unwrap();
                    m.module.update_tray(&dest, tray)
                }
            };
        });

        RefCell::new(TrayCtx {
            module: new_tray_module(config),
            backend_handle: register_tray(s),
        })
    });

    TrayCtxRc(rc)
}
