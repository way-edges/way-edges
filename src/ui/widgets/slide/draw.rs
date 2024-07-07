use std::time::Duration;

use crate::config::Config;
use crate::ui::draws::blur::blur_image_surface;
use crate::ui::draws::transition_state::TransitionState;
use crate::ui::draws::util::draw_frame_manager;
use crate::ui::draws::util::draw_input_region;
use crate::ui::draws::util::draw_motion;
use crate::ui::draws::util::draw_rotation;
use crate::ui::draws::util::new_surface;
use crate::ui::draws::util::Z;

use clap::error::Result;
use gtk::cairo;
use gtk::cairo::Context;
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;

use super::event;
use super::event::Direction;

pub fn setup_draw(window: &gtk::ApplicationWindow, cfg: Config) -> Result<DrawingArea, String> {
    let darea = DrawingArea::new();
    let size = cfg.get_size_into()?;
    let edge = cfg.edge;
    let direction = Direction::Forward;
    let extra_trigger_size = 5.;
    let f_map_size = (size.0 + extra_trigger_size, size.1);
    let map_size = (f_map_size.0 as i32, f_map_size.1 as i32);
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

    let transition_range = (3., size.0);
    let ts = TransitionState::new(Duration::from_millis(100), transition_range);
    let progress = event::setup_event(&darea, &ts, edge.into(), direction, size.1);

    let setup_core = draw_core(size, map_size, f_map_size, direction, edge)?;
    // let is_pressing = mouse_state.borrow().pressing.clone();
    let set_rotate = draw_rotation(edge, size);
    let mut set_motion = draw_motion(edge, transition_range, extra_trigger_size);
    let set_input_region = draw_input_region(size, edge, extra_trigger_size);
    let mut set_frame_manger = draw_frame_manager(60, transition_range);
    darea.set_draw_func(
        glib::clone!(@weak window, @strong progress =>move |darea, context, _, _| {
            set_rotate(context);
            let visible_y = ts.get_y();
            set_motion(context, visible_y);

            let res = setup_core(context, progress.get()).and_then(|_| {
                set_input_region(&window, visible_y).and_then(|_| {
                    set_frame_manger(darea, visible_y, ts.is_forward.get())
                })
            });

            if let Err(e) = res {
                window.close();
                log::error!("{e}");
                crate::notify_send("Way-edges widget draw error", &e, true);
            }
        }),
    );
    window.set_child(Some(&darea));
    Ok(darea)
}

fn draw_core(
    size: (f64, f64),
    map_size: (i32, i32),
    f_map_size: (f64, f64),
    direction: Direction,
    edge: Edge,
) -> Result<impl Fn(&Context, f64) -> Result<(), String>, String> {
    let predraw = super::pre_draw::draw(size, map_size)?;

    fn error_handle(e: cairo::Error) -> String {
        format!("Draw core error: {:?}", e)
    }

    let rotate_progress: Box<dyn Fn(&Context)> = match (edge, direction) {
        (Edge::Left, Direction::Backward)
        | (Edge::Right, Direction::Forward)
        | (Edge::Top, Direction::Forward)
        | (Edge::Bottom, Direction::Backward) => Box::new(move |ctx: &Context| {
            ctx.scale(1., -1.);
            ctx.translate(Z, -f_map_size.1);
        }),
        _ => Box::new(|_: &Context| {}),
    };

    let new_surface = move || new_surface(map_size, error_handle);

    Ok(move |ctx: &Context, progress: f64| -> Result<(), String> {
        let base_surf = {
            let surf = new_surface()?;
            let ctx = cairo::Context::new(&surf).map_err(error_handle)?;
            {
                ctx.set_source_surface(&predraw.bg, Z, Z).unwrap();
                ctx.append_path(&predraw.path);
                ctx.fill().map_err(error_handle)?;
            };
            {
                rotate_progress(&ctx);
                ctx.set_source_surface(&predraw.fg, Z, (progress - 1.) * size.1)
                    .unwrap();
                ctx.append_path(&predraw.path);
                ctx.fill().map_err(error_handle)?;
            };
            surf
        };

        let blur_surface = {
            let mut surf = new_surface()?;
            let ctx = cairo::Context::new(&surf).map_err(error_handle)?;
            ctx.set_source_surface(&base_surf, Z, Z)
                .map_err(error_handle)?;
            ctx.rectangle(Z, Z, f_map_size.0, f_map_size.1);
            ctx.fill().map_err(error_handle)?;
            blur_image_surface(&mut surf, 100)?;
            surf
        };

        ctx.set_source_surface(blur_surface, Z, Z).unwrap();
        ctx.rectangle(Z, Z, f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();

        ctx.set_source_surface(base_surf, Z, Z).unwrap();
        ctx.rectangle(Z, Z, f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();

        ctx.set_source_surface(&predraw.shade, Z, Z).unwrap();
        ctx.rectangle(Z, Z, f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();

        ctx.set_source_surface(&predraw.stroke, Z, Z).unwrap();
        ctx.rectangle(Z, Z, f_map_size.0, f_map_size.1);
        ctx.fill().unwrap();
        Ok(())
    })
}
