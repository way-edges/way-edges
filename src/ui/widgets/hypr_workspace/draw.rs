use std::{cell::Cell, ops::Not, rc::Rc};

use cairo::{Context, ImageSurface, LinearGradient};
use gtk::{
    gdk::RGBA,
    glib,
    prelude::{GdkCairoContextExt, WidgetExt},
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
        transition_state::{self, TransitionStateList, TransitionStateRc},
        util::{color_transition, draw_motion, draw_rotation, new_surface, Z},
    },
};

pub struct DrawCore {
    data: Rc<Cell<HyprGlobalData>>,

    edge: Edge,
    thickness: i32,
    length: i32,
    gap: i32,
    active_increase: f64,

    backlight: Option<RGBA>,
    deactive_color: RGBA,
    active_color: RGBA,

    ts_list: TransitionStateList,
    workspace_transition: TransitionStateRc,
    pop_ts: TransitionStateRc,
    frame_manager: FrameManager,

    // for lifetime usage:
    hypr_event_callback_id: u32,
}

impl Drop for DrawCore {
    fn drop(&mut self) {
        unregister_hypr_event_callback(self.hypr_event_callback_id)
    }
}

impl DrawCore {
    pub fn new(
        darea: &DrawingArea,
        conf: &Config,
        wp_conf: &HyprWorkspaceConfig,
        workspace_transition: TransitionStateRc,
        pop_ts: TransitionStateRc,
        ts_list: TransitionStateList,
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
            move |f| {
                log::debug!("received hyprland worksapce change event: {f:?}");
                data.set(*f);
                {
                    let mut wp_ts = workspace_transition.borrow_mut();
                    let direction = wp_ts.direction;
                    wp_ts.set_direction_self(direction.not());
                }
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

            edge: conf.edge,
            thickness: wp_conf.thickness.get_num_into().unwrap() as i32,
            length: wp_conf.length.get_num_into().unwrap() as i32,
            gap: wp_conf.gap,
            active_increase: wp_conf.active_increase,

            backlight: wp_conf.backlight,
            deactive_color: wp_conf.deactive_color,
            active_color: wp_conf.active_color,

            ts_list,
            workspace_transition,
            pop_ts,
            frame_manager,

            hypr_event_callback_id: id,
        }
    }

    pub fn draw_core(&mut self, ctx: &Context, window: &gtk::ApplicationWindow) {
        self.ts_list.refresh();
        // let range = (0., self.thickness as f64);
        // let y = self.pop_ts.borrow().get_y();
        // let visible_y = transition_state::calculate_transition(y, range);
        draw_rotation(ctx, self.edge, (self.thickness as f64, self.length as f64));
        // draw_motion(ctx, visible_y, range);
        let content = self.draw();
        ctx.set_source_surface(content, Z, Z).unwrap();
        ctx.paint().unwrap();
        self.frame_manager.ensure_frame_run(&self.ts_list);
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

        let a: Vec<(f64, RGBA)> = (1..=data.max_workspace)
            .map(|w| {
                if w == data.current_workspace {
                    (
                        item_min_length + (item_max_length - item_min_length) * y,
                        color_transition(self.deactive_color, self.active_color, y as f32),
                    )
                } else if w == data.last_workspace {
                    (
                        item_min_length + (item_max_length - item_min_length) * (1. - y),
                        color_transition(self.active_color, self.deactive_color, y as f32),
                    )
                } else {
                    (item_min_length, self.deactive_color)
                }
            })
            .collect();

        a.iter().enumerate().for_each(|(index, (t, color))| {
            if index != 0 {
                ctx.translate(Z, self.gap as f64);
            }
            ctx.set_source_color(color);
            ctx.rectangle(Z, Z, self.thickness as f64, *t);
            ctx.fill().unwrap();

            ctx.translate(Z, *t);
        });

        surf
    }
}
