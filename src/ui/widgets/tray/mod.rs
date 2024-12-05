mod layout;
mod module;

use cairo::ImageSurface;
use std::{cell::RefCell, rc::Rc};

use super::wrapbox::{display::grid::DisplayWidget, expose::BoxExpose};
use crate::plug::tray::{icon::parse_icon_given_name, register_tray, unregister_tray};
use module::TrayModule;

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
