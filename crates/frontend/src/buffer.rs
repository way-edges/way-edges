use std::cell::UnsafeCell;
use std::rc::{Rc, Weak};

use cairo::ImageSurface;
use gtk::glib;
use util::draw::new_surface;

pub struct BufferWeak(Weak<UnsafeCell<Option<ImageSurface>>>);
impl glib::clone::Upgrade for BufferWeak {
    type Strong = Buffer;
    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(Buffer)
    }
}
impl glib::clone::Downgrade for Buffer {
    type Weak = BufferWeak;
    fn downgrade(&self) -> Self::Weak {
        BufferWeak(self.0.downgrade())
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self(Rc::new(UnsafeCell::new(None)))
    }
}

#[derive(Clone, Debug)]
// pub struct Buffer(Rc<UnsafeCell<ImageSurface>>);
pub struct Buffer(Rc<UnsafeCell<Option<ImageSurface>>>);

impl Buffer {
    pub fn update_buffer(&self, new: ImageSurface) {
        unsafe { *self.0.get().as_mut().unwrap() = Some(new) }
    }
    pub fn get_buffer(&self) -> ImageSurface {
        unsafe {
            self.0
                .get()
                .as_ref()
                .unwrap()
                .clone()
                .unwrap_or(new_surface((0, 0)))
        }
    }
}
