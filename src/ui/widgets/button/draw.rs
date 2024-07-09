use crate::config::Config;
use crate::ui::draws::transition_state::TransitionState;
use crate::ui::draws::util::draw_frame_manager;
use crate::ui::draws::util::draw_input_region;
use crate::ui::draws::util::draw_motion;
use crate::ui::draws::util::draw_rotation;

use super::event::*;
use super::BtnConfig;
use clap::error::Result;
use gtk::cairo;
use gtk::cairo::Context;
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;
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
    let transition_range = (0., size.0);
    let ts = TransitionState::new(
        Duration::from_millis(btn_cfg.transition_duration),
        transition_range,
    );
    // mouse_state need to change something inside transition state
    // and i want to avoid using refcell, so i make every thing needed inside TransitionState to RcCell
    let mouse_state = setup_event(
        &darea,
        btn_cfg.event_map.take().ok_or("EventMap is None")?,
        &ts,
    );
    let is_pressing = mouse_state.borrow().pressing.clone();
    let set_rotate = draw_rotation(edge, size);
    let mut set_motion = draw_motion(edge, transition_range, extra_trigger_size);
    let set_core = draw_core(map_size, size, btn_cfg.color, extra_trigger_size)?;
    let set_input_region = draw_input_region(size, edge, extra_trigger_size);
    let mut set_frame_manger =
        draw_frame_manager(btn_cfg.frame_rate, transition_range, &darea, window);
    darea.set_draw_func(glib::clone!(@weak window =>move |_, context, _, _| {
        set_rotate(context);
        let visible_y = ts.get_y();
        set_motion(context, visible_y);
        let res = set_core(context, is_pressing.get().is_some()).and_then(|_| {
            set_input_region(&window, visible_y).and_then(|_| {
                set_frame_manger(visible_y, ts.is_forward.get())
            })
        });
        if let Err(e) = res {
            window.close();
            log::error!("{e}");
            // error ignored
            notify_rust::Notification::new().summary("Way-edges widget draw error").body(&e).show().ok();
        }
    }));
    window.set_child(Some(&darea));
    Ok(darea)
}

fn draw_core(
    map_size: (i32, i32),
    size: (f64, f64),
    color: RGBA,
    extra_trigger_size: f64,
) -> Result<impl Fn(&Context, bool) -> Result<(), String>, String> {
    let (b, n, p) = super::pre_draw::draw_to_surface(map_size, size, color, extra_trigger_size)?;
    let f_map_size = (map_size.0 as f64, map_size.1 as f64);

    fn error_handle(e: cairo::Error) -> String {
        format!("Draw core error: {:?}", e)
    }

    Ok(move |ctx: &Context, pressing: bool| -> Result<(), String> {
        // base_surface
        ctx.set_source_surface(&b, 0., 0.).map_err(error_handle)?;
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().map_err(error_handle)?;

        // mask
        if pressing {
            ctx.set_source_surface(&p, 0., 0.)
        } else {
            ctx.set_source_surface(&n, 0., 0.)
        }
        .map_err(error_handle)?;
        ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
        ctx.fill().map_err(error_handle)?;
        Ok(())
    })
}
