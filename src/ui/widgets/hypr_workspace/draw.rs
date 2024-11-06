use std::{
    cell::{Cell, RefCell},
    ops::Not,
    rc::Rc,
};

use cairo::{Context, ImageSurface, RectangleInt, Region};
use gtk::{
    gdk::RGBA,
    glib,
    prelude::{GdkCairoContextExt, NativeExt, SurfaceExt, WidgetExt},
    DrawingArea,
};
use gtk4_layer_shell::Edge;

use crate::{
    config::{widgets::hypr_workspace::HyprWorkspaceConfig, Config},
    plug::hypr_workspace::{
        register_hypr_event_callback, unregister_hypr_event_callback, HyprGlobalData,
    },
    ui::draws::{
        frame_manager::{FrameManager, FrameManagerBindTransition},
        mouse_state::MouseStateRc,
        transition_state::{TransitionStateList, TransitionStateRc},
        util::{color_mix, color_transition, new_surface, Z},
    },
};

pub struct HoverData {
    // [[0, 2], [4, 9]]
    //    2       5
    item_location: Vec<[f64; 2]>,
    edge: Edge,
    pub hover_id: isize,
}

impl HoverData {
    pub fn new(edge: Edge) -> Self {
        Self {
            item_location: vec![],
            edge,
            hover_id: -1,
        }
    }

    pub fn update_hover_data(&mut self, item_location: Vec<[f64; 2]>) {
        self.item_location = item_location;
    }

    pub fn match_hover_id(&self, mouse_pos: (f64, f64)) -> isize {
        let id = match self.edge {
            // Edge::Top | Edge::Bottom => self.item_location.len() as isize - 1 - self.m(mouse_pos.1),
            Edge::Top | Edge::Bottom => self.m(mouse_pos.0),
            Edge::Left | Edge::Right => self.m(mouse_pos.1),
            _ => unreachable!(),
        };
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

    // binary search
    fn m(&self, pos: f64) -> isize {
        if self.item_location.is_empty() {
            return -1;
        }

        let mut index = self.item_location.len() - 1;
        let mut half = self.item_location.len();

        fn half_index(index: &mut usize, half: &mut usize, is_left: bool) {
            *half = (*half / 2).max(1);

            if is_left {
                *index -= *half
            } else {
                *index += *half
            }
        }

        half_index(&mut index, &mut half, true);

        loop {
            let current = self.item_location[index];

            if pos < current[0] {
                if index == 0 || self.item_location[index - 1][1] <= pos {
                    return -1;
                } else {
                    half_index(&mut index, &mut half, true);
                }
            } else if pos >= current[1] {
                if index == self.item_location.len() - 1 || pos < self.item_location[index + 1][0] {
                    return -1;
                } else {
                    half_index(&mut index, &mut half, false);
                }
            } else {
                return index as isize;
            }
        }
    }
}

pub struct DrawCore {
    data: Rc<Cell<HyprGlobalData>>,

    hover_data: Rc<RefCell<HoverData>>,

    edge: Edge,
    thickness: i32,
    length: i32,
    gap: i32,
    active_increase: f64,
    extra_trigger_size: f64,

    deactive_color: RGBA,
    active_color: RGBA,
    hover_color: Option<RGBA>,

    workspace_transition: TransitionStateRc,
    ts_list: TransitionStateList,
    pop_ts: TransitionStateRc,
    frame_manager: FrameManager,

    // for lifetime usage(drop):
    hypr_event_callback_id: u32,
}

impl Drop for DrawCore {
    fn drop(&mut self) {
        log::info!("drop hyprland workspace draw core");
        unregister_hypr_event_callback(self.hypr_event_callback_id)
    }
}

// default horizontal shape(bottom or top)
// rotate clockwise 90 degree to fit for left and right
impl DrawCore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        darea: &DrawingArea,
        conf: &Config,
        wp_conf: &HyprWorkspaceConfig,

        workspace_transition: TransitionStateRc,
        pop_ts: TransitionStateRc,
        ts_list: TransitionStateList,
        hover_data: Rc<RefCell<HoverData>>,

        ms: &MouseStateRc,
    ) -> Self {
        match conf.edge {
            gtk4_layer_shell::Edge::Left | gtk4_layer_shell::Edge::Right => {
                darea.set_size_request(
                    wp_conf.thickness.get_num().unwrap().ceil() as i32,
                    wp_conf.length.get_num().unwrap().ceil() as i32,
                );
            }
            gtk4_layer_shell::Edge::Top | gtk4_layer_shell::Edge::Bottom => {
                darea.set_size_request(
                    wp_conf.length.get_num().unwrap().ceil() as i32,
                    wp_conf.thickness.get_num().unwrap().ceil() as i32,
                );
            }
            _ => todo!(),
        };

        // data related
        let data = Rc::new(Cell::new(HyprGlobalData::default()));
        let (id, init_data) = register_hypr_event_callback(glib::clone!(
            #[weak]
            data,
            #[weak]
            darea,
            #[weak]
            workspace_transition,
            #[weak]
            ms,
            move |f| {
                log::debug!("received hyprland worksapce change event: {f:?}");
                data.set(*f);
                {
                    let mut wp_ts = workspace_transition.borrow_mut();
                    let direction = wp_ts.direction;
                    wp_ts.set_direction_self(direction.not());
                }
                ms.borrow_mut().pop();
                darea.queue_draw();
            }
        ));
        data.set(init_data);

        // frame manager
        let frame_manager = FrameManager::new(
            wp_conf.frame_rate,
            glib::clone!(
                #[weak]
                darea,
                move || {
                    darea.queue_draw();
                }
            ),
        );

        Self {
            data,

            hover_data,

            edge: conf.edge,
            thickness: wp_conf.thickness.get_num_into().unwrap() as i32,
            length: wp_conf.length.get_num_into().unwrap() as i32,
            gap: wp_conf.gap,
            active_increase: wp_conf.active_increase,
            extra_trigger_size: wp_conf.extra_trigger_size.get_num_into().unwrap(),

            deactive_color: wp_conf.deactive_color,
            active_color: wp_conf.active_color,
            hover_color: wp_conf.hover_color,

            ts_list,
            workspace_transition,
            pop_ts,
            frame_manager,

            hypr_event_callback_id: id,
        }
    }

    pub fn draw_core(&mut self, ctx: &Context, window: &gtk::ApplicationWindow) {
        self.ts_list.refresh();
        let y = self.pop_ts.borrow().get_y();
        self.draw_rotation(ctx);
        self.draw_motion(ctx, y);
        let content = self.draw();
        ctx.set_source_surface(content, Z, Z).unwrap();
        ctx.paint().unwrap();

        self.frame_manager.ensure_frame_run(&self.ts_list);
        self.calculate_input_region(y, window);
    }

    fn draw_rotation(&mut self, ctx: &Context) {
        match self.edge {
            Edge::Left | Edge::Right => {
                ctx.rotate(90.0_f64.to_radians());
                ctx.translate(0., -self.length as f64);
            }
            Edge::Top | Edge::Bottom => {}
            _ => unreachable!(),
        }
    }

    fn draw_motion(&self, ctx: &Context, y: f64) {
        let w = self.thickness as f64;
        match self.edge {
            Edge::Left | Edge::Top => {
                ctx.translate(Z, w * (y - 1.));
            }
            Edge::Right | Edge::Bottom => {
                ctx.translate(Z, w * (1. - y));
            }
            _ => unreachable!(),
        }
    }

    fn calculate_input_region(&self, y: f64, window: &gtk::ApplicationWindow) {
        let rect = match self.edge {
            Edge::Left => {
                let w = (self.thickness as f64 * y + self.extra_trigger_size) as i32;
                RectangleInt::new(0, 0, w, self.length)
            }
            Edge::Right => {
                let w = self.thickness as f64 * y + self.extra_trigger_size;
                RectangleInt::new(0, self.thickness - w as i32, w.ceil() as i32, self.length)
            }
            Edge::Top => {
                let w = (self.thickness as f64 * y + self.extra_trigger_size) as i32;
                RectangleInt::new(0, 0, self.length, w)
            }
            Edge::Bottom => {
                let w = self.thickness as f64 * y + self.extra_trigger_size;
                RectangleInt::new(0, self.thickness - w as i32, self.length, w.ceil() as i32)
            }
            _ => todo!(),
        };
        if let Some(s) = window.surface() {
            s.set_input_region(&Region::create_rectangle(&rect))
        }
    }

    pub fn draw(&self) -> ImageSurface {
        let data = self.data.get();
        let item_base_length = {
            let up = (self.length - self.gap * (data.max_workspace - 1)) as f64;
            up / data.max_workspace as f64
        };
        let item_changable_length = item_base_length * self.active_increase;

        let item_max_length = item_base_length + item_changable_length;
        let item_min_length =
            item_base_length - item_changable_length / (data.max_workspace - 1) as f64;

        // let surf = new_surface((self.thickness, self.length));
        let surf = new_surface((self.length, self.thickness));
        let ctx = Context::new(&surf).unwrap();

        let y = self.workspace_transition.borrow().get_abs_y();

        let mut item_location = vec![];
        let mut draw_start_pos = 0.;

        let mut hover_data = self.hover_data.borrow_mut();
        let hover_id = hover_data.hover_id;

        let border_width = self.thickness as f64 / 10.;

        // let a: Vec<(f64, RGBA)> = (1..=data.max_workspace).map(|id| {
        (1..=data.max_workspace).for_each(|id| {
            // size and color
            let (length, mut color) = if id == data.current_workspace {
                (
                    item_min_length + (item_max_length - item_min_length) * y,
                    color_transition(self.deactive_color, self.active_color, y as f32),
                )
            } else if id == data.prev_workspace {
                (
                    item_min_length + (item_max_length - item_min_length) * (1. - y),
                    color_transition(self.active_color, self.deactive_color, y as f32),
                )
            } else {
                (item_min_length, self.deactive_color)
            };

            // mouse hover color
            if let Some(hover_color) = self.hover_color {
                if id as isize == hover_id {
                    color = color_mix(hover_color, color);
                }
            }

            // draw
            if id == data.current_workspace {
                ctx.set_source_color(&color);
                ctx.rectangle(Z, Z, length, self.thickness as f64);
                ctx.fill().unwrap();
            } else {
                ctx.set_source_color(&self.active_color);
                ctx.rectangle(Z, Z, length, self.thickness as f64);
                ctx.fill().unwrap();
                ctx.set_source_color(&color);
                ctx.rectangle(
                    border_width,
                    border_width,
                    length - 2. * border_width,
                    self.thickness as f64 - 2. * border_width,
                );
                ctx.fill().unwrap();
            }
            if id != data.max_workspace {
                ctx.translate(length + self.gap as f64, Z);
            };

            // record calculated size for mouse locate
            let end = draw_start_pos + length;
            item_location.push([draw_start_pos, draw_start_pos + length]);
            draw_start_pos = end + self.gap as f64;
        });

        hover_data.update_hover_data(item_location);

        surf
    }
}
