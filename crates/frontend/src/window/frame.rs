use gtk::DrawingArea;

use crate::animation::{AnimationList, ToggleAnimationRc};
use crate::frame::FrameManager;

#[derive(Debug)]
pub(super) struct WindowFrameManager {
    pop_animation: ToggleAnimationRc,
    animation_list: AnimationList,
    base: FrameManager,

    pop_animation_finished: bool,
    widget_animation_finished: bool,
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

            pop_animation_finished: true,
            widget_animation_finished: true,
        }
    }
    pub(super) fn ensure_animations(&mut self, darea: &DrawingArea) -> bool {
        let widget_has_animation_update = self.animation_list.refresh_and_has_in_progress();
        let pop_animation_update = {
            let mut pop_animation = self.pop_animation.borrow_mut();
            pop_animation.refresh();
            pop_animation.is_in_progress()
        };

        if widget_has_animation_update {
            if self.widget_animation_finished {
                self.widget_animation_finished = false
            }
            self.base.start(darea);
            return true;
        } else if !self.widget_animation_finished {
            self.widget_animation_finished = true;
            self.base.start(darea);
            return true;
        }

        if pop_animation_update {
            if self.pop_animation_finished {
                self.pop_animation_finished = false
            }
            self.base.start(darea);
        } else if !self.pop_animation_finished {
            self.pop_animation_finished = true;
            self.base.start(darea);
        } else {
            self.base.stop();
        }

        false
    }
}
