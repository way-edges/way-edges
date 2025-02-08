use std::{cell::Cell, fmt::Debug, rc::Rc};

use cairo::ImageSurface;
use way_edges_derive::wrap_rc;

use crate::{animation::AnimationList, buffer::Buffer, mouse_state::MouseEvent};

use super::grid::GridBox;

pub type BoxedWidgetGrid = GridBox<BoxedWidgetCtxRc>;

pub trait BoxedWidget: Debug {
    fn content(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, _: MouseEvent) -> bool {
        false
    }
}

#[wrap_rc(rc = "pub", normal = "pub")]
#[derive(Debug)]
pub struct BoxedWidgetCtx {
    pub ctx: Box<dyn BoxedWidget>,
    pub has_update: Rc<Cell<bool>>,
    pub animation_list: AnimationList,
    pub did_last_frame: bool,
    pub buffer: Buffer,
}
impl BoxedWidgetCtx {
    pub fn new(
        ctx: impl BoxedWidget + 'static,
        animation_list: AnimationList,
        has_update: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            ctx: Box::new(ctx),
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
    pub fn draw(&mut self) -> ImageSurface {
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
            let content = self.ctx.content();
            self.update_buffer(content);
        }

        self.get_buffer()
    }
    pub fn on_mouse_event(&mut self, event: MouseEvent) -> bool {
        let should_update = self.ctx.on_mouse_event(event);
        if should_update {
            self.has_update.set(true);
        }
        should_update
    }
}

impl PartialEq for BoxedWidgetCtxRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq for BoxedWidgetCtxRcWeak {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Weak::ptr_eq(&self.0, &other.0)
    }
}
