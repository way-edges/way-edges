use std::{fmt::Debug, rc::Rc};

use cairo::ImageSurface;
use way_edges_derive::wrap_rc;

use crate::{animation::AnimationList, buffer::Buffer, mouse_state::MouseEvent};

use super::grid::{item::GridItemContent, GridBox};

pub type BoxedWidgetGrid = GridBox<BoxedWidgetCtxRc>;

pub trait BoxedWidget: Debug {
    fn has_update(&mut self) -> bool {
        false
    }
    fn content(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, _: MouseEvent) {}
}

#[wrap_rc(rc = "pub", normal = "pub")]
#[derive(Debug)]
pub struct BoxedWidgetCtx {
    ctx: Box<dyn BoxedWidget>,
    animation_list: AnimationList,
    did_last_frame: bool,
    buffer: Buffer,
}
impl BoxedWidgetCtx {
    pub fn new(ctx: impl BoxedWidget + 'static, animation_list: AnimationList) -> Self {
        Self {
            ctx: Box::new(ctx),
            animation_list,
            did_last_frame: true,
            buffer: Buffer::default(),
        }
    }
    fn update_buffer(&mut self, img: ImageSurface) {
        self.buffer.update_buffer(img);
    }
    fn get_buffer(&self) -> ImageSurface {
        self.buffer.get_buffer()
    }
    pub fn on_mouse_event(&mut self, e: MouseEvent) {
        self.ctx.on_mouse_event(e);
    }
}

impl GridItemContent for BoxedWidgetCtxRc {
    fn draw(&mut self) -> ImageSurface {
        let mut ctx = self.borrow_mut();

        let mut call_redraw = false;
        if ctx.animation_list.has_in_progress() {
            if ctx.did_last_frame {
                ctx.did_last_frame = false
            }
            call_redraw = true
        } else if !ctx.did_last_frame {
            ctx.did_last_frame = true;
            call_redraw = true
        } else if ctx.ctx.has_update() {
            call_redraw = true
        }

        if call_redraw {
            let content = ctx.ctx.content();
            ctx.update_buffer(content);
        }

        ctx.get_buffer()
    }
}

impl Eq for BoxedWidgetCtxRc {}
impl PartialEq for BoxedWidgetCtxRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
