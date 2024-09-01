use std::{cell::Cell, ops::Not, rc::Rc};

use cairo::{Context, ImageSurface, LinearGradient, RectangleInt, Region};
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

pub struct DrawData {
    data: Vec<[f64; 2]>, // [[0, 2], [4, 9]]
    //                         2       5
    edge: Edge,
}

impl DrawData {
    pub fn new(edge: Edge) -> Self {
        Self { data: vec![], edge }
    }
    pub fn match_workspace(&self, mouse_pos: (f64, f64)) -> isize {
        match self.edge {
            Edge::Left | Edge::Right => self.data.len() as isize - 1 - self.m(mouse_pos.1),
            Edge::Top | Edge::Bottom => self.m(mouse_pos.0),
            _ => unreachable!(),
        }
    }

    // binary search
    fn m(&self, pos: f64) -> isize {
        if self.data.is_empty() {
            return -1;
        }

        let mut index = self.data.len() / 2;

        loop {
            let current = self.data[index];

            if pos < current[0] {
                if index == 0 || self.data[index - 1][1] <= pos {
                    // reach start || between [last-end, current-start]
                    return -1;
                } else {
                    // div 2
                    index /= 2;
                }
            } else if pos >= current[1] {
                if index == self.data.len() - 1 || pos < self.data[index + 1][0] {
                    // reach end || between [current-end, next-start]
                    return -1;
                } else {
                    // o + div(o+1) 2
                    index = index + ((index + 1) / 2);
                }
            } else {
                return index as isize;
            }
        }
    }
}

pub struct DrawCore {
    data: Rc<Cell<HyprGlobalData>>,

    workspace_draw_data: Rc<Cell<DrawData>>,
    hover_id: Rc<Cell<isize>>,

    edge: Edge,
    thickness: i32,
    length: i32,
    gap: i32,
    active_increase: f64,
    extra_trigger_size: f64,

    backlight: Option<RGBA>,
    deactive_color: RGBA,
    active_color: RGBA,
    hover_color: Option<RGBA>,

    workspace_transition: TransitionStateRc,
    ts_list: TransitionStateList,
    pop_ts: TransitionStateRc,
    frame_manager: FrameManager,

    // for lifetime usage:
    hypr_event_callback_id: u32,
}

impl Drop for DrawCore {
    fn drop(&mut self) {
        log::info!("drop hyprland workspace draw core");
        unregister_hypr_event_callback(self.hypr_event_callback_id)
    }
}

impl DrawCore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        darea: &DrawingArea,
        conf: &Config,
        wp_conf: &HyprWorkspaceConfig,

        workspace_transition: TransitionStateRc,
        pop_ts: TransitionStateRc,
        ts_list: TransitionStateList,
        workspace_draw_data: Rc<Cell<DrawData>>,
        hover_id: Rc<Cell<isize>>,

        ms: &MouseStateRc,
    ) -> Self {
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

            workspace_draw_data,
            hover_id,

            edge: conf.edge,
            thickness: wp_conf.thickness.get_num_into().unwrap() as i32,
            length: wp_conf.length.get_num_into().unwrap() as i32,
            gap: wp_conf.gap,
            active_increase: wp_conf.active_increase,
            extra_trigger_size: wp_conf.extra_trigger_size.get_num_into().unwrap(),

            backlight: wp_conf.backlight,
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
            Edge::Left | Edge::Right => {}
            Edge::Top | Edge::Bottom => {
                ctx.rotate(90.0_f64.to_radians());
                ctx.translate(0., -self.length as f64);
            }
            _ => unreachable!(),
        }
    }

    fn draw_motion(&self, ctx: &Context, y: f64) {
        let w = self.thickness as f64;
        match self.edge {
            Edge::Left | Edge::Top => {
                ctx.translate(w * (y - 1.), Z);
            }
            Edge::Right | Edge::Bottom => {
                ctx.translate(w * (1. - y), Z);
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

        let surf = new_surface((self.thickness, self.length));
        // let surf = ImageSurface::create(Format::ARgb32, self.thickness.ceil() as i32, self.length)
        // .unwrap();
        let ctx = Context::new(&surf).unwrap();

        if let Some(backlight_color) = self.backlight {
            let backlight = LinearGradient::new(
                Z,
                self.length as f64 / 2.,
                self.thickness as f64,
                self.length as f64 / 2.,
            );
            backlight.add_color_stop_rgba(
                Z,
                backlight_color.red().into(),
                backlight_color.green().into(),
                backlight_color.blue().into(),
                0.5,
            );
            backlight.add_color_stop_rgba(
                1.,
                backlight_color.red().into(),
                backlight_color.green().into(),
                backlight_color.blue().into(),
                Z,
            );
            ctx.set_source(backlight).unwrap();
            ctx.paint().unwrap();
        }

        let y = {
            let a = self.workspace_transition.borrow();
            a.get_abs_y()
        };

        let mut draw_data = DrawData::new(self.edge);
        let mut draw_start_pos = 0.;

        let hover_id = self.hover_id.get();

        let a: Vec<(f64, RGBA)> = (1..=data.max_workspace)
            .map(|id| {
                let (size, mut color) = if id == data.current_workspace {
                    (
                        item_min_length + (item_max_length - item_min_length) * y,
                        color_transition(self.deactive_color, self.active_color, y as f32),
                    )
                } else if id == data.last_workspace {
                    (
                        item_min_length + (item_max_length - item_min_length) * (1. - y),
                        color_transition(self.active_color, self.deactive_color, y as f32),
                    )
                } else {
                    (item_min_length, self.deactive_color)
                };

                if let Some(hover_color) = self.hover_color {
                    if id as isize == hover_id {
                        color = color_mix(hover_color, color);
                    }
                }

                {
                    let end = draw_start_pos + size;
                    draw_data.data.push([draw_start_pos, draw_start_pos + size]);
                    draw_start_pos = end + self.gap as f64;
                }

                (size, color)
            })
            .collect();

        a.iter().rev().enumerate().for_each(|(index, (t, color))| {
            if index != 0 {
                ctx.translate(Z, self.gap as f64);
            }
            ctx.set_source_color(color);
            ctx.rectangle(Z, Z, self.thickness as f64, *t);
            ctx.fill().unwrap();

            ctx.translate(Z, *t);
        });

        self.workspace_draw_data.set(draw_data);

        surf
    }
}
