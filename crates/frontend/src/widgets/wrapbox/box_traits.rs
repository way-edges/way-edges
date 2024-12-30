use std::{cell::RefCell, fmt::Debug, rc::Rc};

use cairo::ImageSurface;

use crate::mouse_state::MouseEvent;

use super::grid::GridBox;

pub trait BoxedWidget: Debug {
    fn content(&mut self) -> Option<ImageSurface> {
        None
    }
    fn on_mouse_event(&mut self, _: MouseEvent) {}
}

impl GridBox<Rc<RefCell<dyn BoxedWidget>>> {
    pub fn draw_content(&mut self) -> ImageSurface {
        self.draw(|widget| widget.borrow_mut().content())
    }
}
