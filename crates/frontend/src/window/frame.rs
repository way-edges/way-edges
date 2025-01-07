use gtk::DrawingArea;
use way_edges_derive::wrap_rc;

use crate::animation::{AnimationList, ToggleAnimationRc};
use crate::frame::FrameManager;

#[wrap_rc(rc = "pub(super)", normal = "pub(super)")]
#[derive(Debug)]
pub(super) struct WindowFrameManager {
    pop_animation: ToggleAnimationRc,
    animation_list: AnimationList,
    base: FrameManager,

    animation_finished: bool,
}
impl WindowFrameManager {
    pub(super) fn new(
        frame_rate: u64,
        animation_list: AnimationList,
        pop_animation: ToggleAnimationRc,
    ) -> Self {
        Self {
            pop_animation,
            animation_list,
            base: FrameManager::new(frame_rate),

            animation_finished: true,
        }
    }
    pub(super) fn ensure_animations(&mut self, darea: &DrawingArea) {
        if self.animation_list.refresh_and_has_in_progress() || {
            let mut pop_animation = self.pop_animation.borrow_mut();
            pop_animation.refresh();
            pop_animation.is_in_progress()
        } {
            self.base.start(darea);
            if self.animation_finished {
                self.animation_finished = false
            }
        } else if !self.animation_finished {
            self.animation_finished = true;
            self.base.start(darea);
        } else {
            self.base.stop();
        }
    }
}
