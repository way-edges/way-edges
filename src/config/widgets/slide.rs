use educe::Educe;
use gtk::gdk::RGBA;

use crate::config::NumOrRelative;

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Educe)]
#[educe(Debug)]
pub struct SlideConfig {
    pub transition_duration: u64,
    pub frame_rate: u64,
    pub extra_trigger_size: NumOrRelative,

    pub bg_color: RGBA,
    pub fg_color: RGBA,
    pub border_color: RGBA,
    pub text_color: RGBA,
    pub is_text_position_start: bool,
    pub preview_size: bool,
    pub progress_direction: Direction,
    #[educe(Debug(ignore))]
    pub on_change: Box<dyn FnMut() + 'static>,
}
