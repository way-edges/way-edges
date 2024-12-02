use super::wrapbox::{display::grid::DisplayWidget, expose::BoxExposeRc};

struct Tray {}

struct TrayModule {}

struct TrayCtx {
    module: TrayModule,
    content: cairo::ImageSurface,
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

pub fn init_tray(expose: &BoxExposeRc) {
    let update_func = expose.borrow_mut().update_func();
}
