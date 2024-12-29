use backend::hypr_workspace::change_to_workspace;
use config::widgets::hypr_workspace::HyprWorkspaceConfig;
use gtk::gdk::BUTTON_PRIMARY;
use gtk4_layer_shell::Edge;
use util::binary_search_within_range;
use way_edges_derive::wrap_rc;

type ItemLocation = Vec<[f64; 2]>;
type MatchItemFunc = fn(&ItemLocation, (f64, f64)) -> isize;

fn make_hover_match_func(edge: Edge) -> MatchItemFunc {
    macro_rules! create_func {
        ($name:ident, $i:tt) => {
            fn $name(item_location: &ItemLocation, mouse_pos: (f64, f64)) -> isize {
                binary_search_within_range(item_location, mouse_pos.$i)
            }
        };
    }
    create_func!(h, 0);
    create_func!(v, 1);
    match edge {
        Edge::Top | Edge::Bottom => h,
        Edge::Left | Edge::Right => v,
        _ => unreachable!(),
    }
}

#[wrap_rc(rc = "pub(super)", normal = "pub(super)")]
#[derive(Debug)]
pub struct HoverData {
    // [[0, 2], [4, 9]]
    //    2       5
    item_location: Vec<[f64; 2]>,
    match_item_func: MatchItemFunc,
    pub hover_id: isize,
}

impl HoverData {
    pub fn new(edge: Edge) -> Self {
        Self {
            item_location: vec![],
            match_item_func: make_hover_match_func(edge),
            hover_id: -1,
        }
    }

    pub fn update_hover_data(&mut self, item_location: Vec<[f64; 2]>) {
        self.item_location = item_location;
    }

    pub fn match_hover_id(&self, mouse_pos: (f64, f64)) -> isize {
        let id = (self.match_item_func)(&self.item_location, mouse_pos);
        if id < 0 {
            id
        } else {
            // to match workspace id
            id + 1
        }
    }

    pub fn update_hover_id_with_mouse_position(&mut self, mouse_pos: (f64, f64)) -> isize {
        self.hover_id = self.match_hover_id(mouse_pos);
        self.hover_id
    }

    pub fn force_update_hover_id(&mut self, id: isize) {
        self.hover_id = id
    }
}

use crate::mouse_state::MouseEvent;
use crate::window::WindowContext;
use config::Config;

pub(super) fn setup_event(
    window: &mut WindowContext,
    conf: &Config,
    w_conf: &mut HyprWorkspaceConfig,
    hover_data: HoverDataRc,
) {
    window.setup_mouse_event_callback(move |_, event| {
        let mut should_redraw = false;
        macro_rules! hhh {
            ($hover_data:expr, $pos:expr) => {{
                let mut h = $hover_data.borrow_mut();
                let old = h.hover_id;
                h.update_hover_id_with_mouse_position($pos) != old
            }};
        }
        match event {
            MouseEvent::Release(pos, key) => {
                if key == BUTTON_PRIMARY {
                    should_redraw = hhh!(hover_data, pos);
                    let id = hover_data.borrow().hover_id;
                    if id > 0 {
                        change_to_workspace(id as i32);
                    }
                };
            }
            MouseEvent::Enter(pos) => {
                should_redraw = hhh!(hover_data, pos);
            }
            MouseEvent::Motion(pos) => {
                should_redraw = hhh!(hover_data, pos);
            }
            MouseEvent::Leave => {
                let mut h = hover_data.borrow_mut();
                let old = h.hover_id;
                if old != -1 {
                    h.force_update_hover_id(-1);
                    should_redraw = true;
                }
            }
            _ => {}
        };
        should_redraw
    });
}
