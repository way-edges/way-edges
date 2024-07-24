use crate::config::Config;
use crate::ui::draws::mouse_state::MouseStateRc;
use crate::ui::draws::transition_state;
use crate::ui::draws::transition_state::TransitionState;
use crate::ui::draws::util::draw_frame_manager;
use crate::ui::draws::util::draw_input_region;
use crate::ui::draws::util::draw_motion;
use crate::ui::draws::util::draw_rotation;

use super::event::*;
use super::BtnConfig;
use clap::error::Result;
use gtk::cairo::Context;
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn setup_draw(
    window: &gtk::ApplicationWindow,
    cfg: Config,
    mut btn_cfg: BtnConfig,
) -> Result<DrawingArea, String> {
    let darea = DrawingArea::new();
    let size = btn_cfg.get_size()?;
    let edge = cfg.edge;
    let extra_trigger_size = btn_cfg.extra_trigger_size.get_num_into()?;
    let map_size = ((size.0 + extra_trigger_size) as i32, size.1 as i32);
    match edge {
        Edge::Left | Edge::Right => {
            darea.set_width_request(map_size.0);
            darea.set_height_request(map_size.1);
        }
        Edge::Top | Edge::Bottom => {
            darea.set_width_request(map_size.1);
            darea.set_height_request(map_size.0);
        }
        _ => unreachable!(),
    };

    // visible range is 0 -> width
    let ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        btn_cfg.transition_duration,
    ))));
    let ms = setup_event(
        &darea,
        btn_cfg.event_map.take().ok_or("EventMap is None")?,
        ts.clone(),
    );
    let mut dc = DrawCore::new(&darea, window, &cfg, &btn_cfg, ms);
    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |_, context, _, _| {
            let visible_y = ts.borrow().get_y();
            dc.draw(context, visible_y, &window);
        }
    ));
    window.set_child(Some(&darea));
    Ok(darea)
}

struct DrawCore {
    rotate: Box<dyn Fn(&Context)>,
    motion: Box<dyn FnMut(&Context, f64)>,
    core: Box<dyn Fn(&Context, bool)>,
    input_region: Box<dyn Fn(&gtk::ApplicationWindow, f64) -> Result<(), String>>,
    frame_manger: Box<dyn FnMut(f64) -> Result<(), String>>,

    ms: MouseStateRc,
    transition_range: (f64, f64),
}

impl DrawCore {
    fn new(
        darea: &DrawingArea,
        window: &gtk::ApplicationWindow,
        cfg: &Config,
        btn_cfg: &BtnConfig,
        ms: MouseStateRc,
    ) -> Self {
        let size = btn_cfg.get_size().unwrap();
        let edge = cfg.edge;
        let extra_trigger_size = btn_cfg.extra_trigger_size.get_num_into().unwrap();
        let map_size = ((size.0 + extra_trigger_size) as i32, size.1 as i32);
        let transition_range = (0., size.0);

        let rotate = draw_rotation(edge, size);
        let motion = Box::new(draw_motion(edge, transition_range, extra_trigger_size));
        let core = Box::new(draw_core(map_size, size, btn_cfg.color, extra_trigger_size));
        let input_region = Box::new(draw_input_region(size, edge, extra_trigger_size));
        let frame_manger = Box::new(draw_frame_manager(btn_cfg.frame_rate, darea, window));
        Self {
            rotate,
            motion,
            core,
            input_region,
            frame_manger,
            ms,
            transition_range,
        }
    }
    fn draw(&mut self, context: &Context, y: f64, window: &gtk::ApplicationWindow) {
        let visible_y = transition_state::calculate_transition(y, self.transition_range);
        (self.rotate)(context);
        (self.motion)(context, visible_y);
        let is_pressing = self.ms.borrow().pressing.is_some();
        println!("is_pressing: {is_pressing}");
        (self.core)(context, is_pressing);
        let res = (self.input_region)(window, visible_y).and_then(|_| (self.frame_manger)(y));
        if let Err(e) = res {
            window.close();
            log::error!("{e}");
            // error ignored
            notify_rust::Notification::new()
                .summary("Way-edges widget draw error")
                .body(&e)
                .show()
                .ok();
        }
    }
}

fn draw_core(
    map_size: (i32, i32),
    size: (f64, f64),
    color: RGBA,
    extra_trigger_size: f64,
) -> impl Fn(&Context, bool) {
    let (b, n, p) = super::pre_draw::draw_to_surface(map_size, size, color, extra_trigger_size);
    let f_map_size = (map_size.0 as f64, map_size.1 as f64);

    move |ctx: &Context, pressing: bool| {
        // base_surface
        ctx.set_source_surface(&b, 0., 0.).unwrap();
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();

        // mask
        if pressing {
            ctx.set_source_surface(&p, 0., 0.)
        } else {
            ctx.set_source_surface(&n, 0., 0.)
        }
        .unwrap();
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();
    }
}
