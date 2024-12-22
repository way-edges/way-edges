use gtk::DrawingArea;
use way_edges_derive::wrap_rc;

use crate::animation::AnimationListRc;
use crate::frame::FrameManager;

#[wrap_rc(rc = "pub(super)", normal = "pub(super)")]
#[derive(Debug)]
pub(super) struct WindowFrameManager {
    animation_list: AnimationListRc,
    base: FrameManager,
}
impl WindowFrameManager {
    pub(super) fn new(frame_rate: u64, animation_list: AnimationListRc) -> Self {
        Self {
            animation_list,
            base: FrameManager::new(frame_rate),
        }
    }
    pub(super) fn refresh_animations(&self) {
        self.animation_list.borrow_mut().refresh();
    }
    pub(super) fn ensure_animations(&mut self, darea: &DrawingArea) {
        if self
            .animation_list
            .borrow_mut()
            .refresh_and_has_in_progress()
        {
            self.base.start(darea);
        } else {
            self.base.stop();
        }
    }
}
