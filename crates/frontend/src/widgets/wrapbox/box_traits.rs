use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
};

use cairo::ImageSurface;

use crate::{animation::AnimationList, buffer::Buffer, mouse_state::MouseEvent};

use super::grid::{item::GridItemContent, GridBox};

pub type BoxedWidgetRc = Rc<RefCell<dyn BoxedWidget>>;
pub type BoxedWidgetGrid = GridBox<BoxedWidgetCtx>;

pub trait BoxedWidget: Debug {
    fn content(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, _: MouseEvent) {}
}

#[derive(Debug)]
pub struct BoxedWidgetCtx {
    pub ctx: BoxedWidgetRc,
    has_update: Rc<Cell<bool>>,
    animation_list: AnimationList,
    did_last_frame: bool,
    buffer: Buffer,
}
impl BoxedWidgetCtx {
    pub fn new(
        ctx: impl BoxedWidget + 'static,
        animation_list: AnimationList,
        has_update: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            ctx: Rc::new(RefCell::new(ctx)),
            animation_list,
            did_last_frame: true,
            buffer: Buffer::default(),
            has_update,
        }
    }
    fn update_buffer(&mut self, img: ImageSurface) {
        self.buffer.update_buffer(img);
    }
    fn get_buffer(&self) -> ImageSurface {
        self.buffer.get_buffer()
    }
}

impl GridItemContent for BoxedWidgetCtx {
    fn has_update(&mut self) -> bool {
        !self.did_last_frame || self.has_update.get() || self.animation_list.has_in_progress()
    }
    fn draw(&mut self) -> ImageSurface {
        let mut call_redraw = false;
        if self.animation_list.has_in_progress() {
            if self.did_last_frame {
                self.did_last_frame = false
            }
            call_redraw = true
        } else if !self.did_last_frame {
            self.did_last_frame = true;
            call_redraw = true
        } else if self.has_update.get() {
            self.has_update.set(false);
            call_redraw = true
        }

        if call_redraw {
            let content = self.ctx.borrow_mut().content();
            self.update_buffer(content);
        }

        self.get_buffer()
    }
}
