use crate::ui::draws::frame_manager::{FrameManager, FrameManagerBindTransition};
use crate::ui::draws::mouse_state::MouseStateRc;
use crate::ui::draws::transition_state::{self, TransitionStateList, TransitionStateRc};
use crate::ui::draws::util::{draw_motion, draw_rotation, ensure_input_region};
use config::Config;

use super::pre_draw::PreDrawCache;
use super::BtnConfig;
use clap::error::Result;
use gtk::cairo::Context;
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;

pub fn setup_draw(
    window: &gtk::ApplicationWindow,
    darea: &gtk::DrawingArea,
    cfg: Config,
    btn_cfg: BtnConfig,
    mouse_state: MouseStateRc,
    transition_state_list: TransitionStateList,
    popup_transition: TransitionStateRc,
) -> Result<(), String> {
    let mut dc = DrawCore::new(
        darea,
        &cfg,
        &btn_cfg,
        mouse_state,
        transition_state_list,
        popup_transition,
    );
    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |_, context, _, _| {
            dc.draw(context, &window);
        }
    ));

    Ok(())
}

struct DrawCore {
    predraw_cache: PreDrawCache,
    frame_manager: FrameManager,
    ts_list: TransitionStateList,
    pop_ts: TransitionStateRc,

    ms: MouseStateRc,
    transition_range: (f64, f64),
    f_map_size: (f64, f64),
    size: (f64, f64),
    edge: Edge,
    extra_trigger_size: f64,
}

impl DrawCore {
    fn new(
        darea: &DrawingArea,
        cfg: &Config,
        btn_cfg: &BtnConfig,
        ms: MouseStateRc,
        ts_list: TransitionStateList,
        pop_ts: TransitionStateRc,
    ) -> Self {
        let size = btn_cfg.size().unwrap();
        let edge = cfg.edge;
        let extra_trigger_size = btn_cfg.extra_trigger_size.get_num_into().unwrap();
        let f_map_size = ((size.0 + extra_trigger_size), size.1);
        let map_size = (f_map_size.0.ceil() as i32, f_map_size.1.ceil() as i32);
        let transition_range = (0., size.0);

        let predraw_cache =
            super::pre_draw::draw_to_surface(map_size, size, btn_cfg.color, extra_trigger_size);

        let frame_manager = FrameManager::new(
            btn_cfg.frame_rate,
            glib::clone!(
                #[weak]
                darea,
                move || {
                    darea.queue_draw();
                }
            ),
        );

        Self {
            predraw_cache,
            frame_manager,
            ts_list,
            pop_ts,

            ms,

            transition_range,
            f_map_size,
            size,
            extra_trigger_size,
            edge,
        }
    }

    fn draw(&mut self, context: &Context, window: &gtk::ApplicationWindow) {
        let y = {
            self.ts_list.refresh();
            self.pop_ts.borrow().get_y()
        };

        let visible_y = transition_state::calculate_transition(y, self.transition_range);
        draw_rotation(context, self.edge, self.size);
        draw_motion(context, visible_y, self.transition_range);
        let is_pressing = self.ms.borrow().pressing.is_some();

        self.draw_core(context, is_pressing, self.f_map_size);

        ensure_input_region(
            window,
            visible_y,
            self.size,
            self.edge,
            self.extra_trigger_size,
        );
        self.frame_manager.ensure_frame_run(&self.ts_list);
    }

    fn draw_core(&self, ctx: &Context, pressing: bool, f_map_size: (f64, f64)) {
        // base_surface
        ctx.set_source_surface(&self.predraw_cache.base_surf, 0., 0.)
            .unwrap();
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();

        // mask
        if pressing {
            ctx.set_source_surface(&self.predraw_cache.press_state_shadow[1], 0., 0.)
        } else {
            ctx.set_source_surface(&self.predraw_cache.press_state_shadow[0], 0., 0.)
        }
        .unwrap();
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();
    }
}
