use cairo::ImageSurface;

use crate::mouse_state::{MouseEvent, MouseStateData};

pub mod button;
pub mod slide;
pub mod workspace;
pub mod wrapbox;

pub trait WidgetContext: std::fmt::Debug {
    fn redraw(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, data: &MouseStateData, event: MouseEvent) -> bool;
}
