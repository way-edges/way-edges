use smithay_client_toolkit::shell::wlr_layer::Anchor;
use util::binary_search_within_range;
use way_edges_derive::wrap_rc;

type ItemLocation = Vec<[f64; 2]>;
type MatchItemFunc = fn(&ItemLocation, (f64, f64)) -> isize;

fn make_hover_match_func(edge: Anchor) -> MatchItemFunc {
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
        Anchor::TOP | Anchor::BOTTOM => h,
        Anchor::LEFT | Anchor::RIGHT => v,
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
    invert_direction: bool,
    pub hover_id: isize,
}

impl HoverData {
    pub fn new(edge: Anchor, invert_direction: bool) -> Self {
        Self {
            item_location: vec![],
            match_item_func: make_hover_match_func(edge),
            hover_id: -1,
            invert_direction,
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
            if self.invert_direction {
                self.item_location.len() as isize - id
            } else {
                id + 1
            }
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
