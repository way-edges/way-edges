use std::fs::File;
use std::time::Duration;

use crate::config::Config;
use crate::ui::draws::blur::blur_image_surface;
use crate::ui::draws::font::get_font_face;
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::transition_state::TransitionState;
use crate::ui::draws::util::draw_frame_manager_now;
use crate::ui::draws::util::draw_input_region_now;
use crate::ui::draws::util::draw_motion_now;
use crate::ui::draws::util::draw_rotation_now;
use crate::ui::draws::util::new_surface;
use crate::ui::draws::util::Z;

use cairo::ImageSurface;
use gtk::cairo;
use gtk::cairo::Context;
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;

use super::event;
use super::event::Direction;
use super::pre_draw::SlidePredraw;

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
    let is_start = false;

    let predraw = super::pre_draw::draw(size, map_size)?;
    let dc = DrawCore {
        predraw,
        edge,
        direction,
        size,
        f_map_size,
        map_size,
        extra_trigger_size,
        is_start,
    };
    let mut frame_manager = FrameManager::new(30);
    // let set_rotate = draw_rotation(edge, size);
    // let mut set_motion = draw_motion(edge, transition_range, extra_trigger_size);
    // let set_input_region = draw_input_region(size, edge, extra_trigger_size);
    // let mut set_frame_manger = draw_frame_manager(60, transition_range);
    darea.set_draw_func(glib::clone!(
        @weak window,
        @strong progress,
        => move |darea, context, _, _| {
            draw_rotation_now(context, dc.edge, dc.size);
            // draw_rotation_now(context, Edge::Top, dc.size);
            let visible_y = ts.get_y();
            draw_motion_now(context, visible_y, dc.edge, transition_range, dc.extra_trigger_size);

            let res = dc.draw(
                context,
                progress.get(),
            ).and_then(|_| {
                draw_input_region_now(
                        &window,
                        visible_y,
                        dc.size,
                        dc.edge,
                        dc.extra_trigger_size
                    ).and_then(|_| {
                        draw_frame_manager_now(
                            darea,
                            &mut frame_manager,
                            visible_y,
                            ts.is_forward.get(),
                            transition_range
                        )
                    })
            });

            if let Err(e) = res {
                window.close();
                log::error!("{e}");
                crate::notify_send("Way-edges widget draw error", &e, true);
            }
    }));
    window.set_child(Some(&darea));
    Ok(darea)
}

struct DrawCore {
    predraw: SlidePredraw,
    edge: Edge,
    direction: Direction,
    size: (f64, f64),
    f_map_size: (f64, f64),
    map_size: (i32, i32),
    extra_trigger_size: f64,
    is_start: bool,
}
impl DrawCore {
    fn draw(&self, ctx: &Context, progress: f64) -> Result<(), String> {
        fn error_handle(e: cairo::Error) -> String {
            format!("Draw core error: {:?}", e)
        }
        let base_surf = {
            let surf = self.new_surface()?;
            let ctx = cairo::Context::new(&surf).map_err(error_handle)?;
            {
                ctx.set_source_surface(&self.predraw.bg, Z, Z).unwrap();
                ctx.append_path(&self.predraw.path);
                ctx.fill().map_err(error_handle)?;
            };
            {
                // rotate progress
                match (self.edge, self.direction) {
                    (Edge::Left, Direction::Backward)
                    | (Edge::Right, Direction::Forward)
                    | (Edge::Top, Direction::Forward)
                    | (Edge::Bottom, Direction::Backward) => {
                        ctx.scale(1., -1.);
                        ctx.translate(Z, -self.f_map_size.1);
                    }
                    _ => {}
                }
                ctx.set_source_surface(&self.predraw.fg, Z, (progress - 1.) * self.size.1)
                    .unwrap();
                ctx.append_path(&self.predraw.path);
                ctx.fill().map_err(error_handle)?;
            };
            surf
        };

        let blur_surface = {
            let mut surf = self.new_surface()?;
            let ctx = cairo::Context::new(&surf).map_err(error_handle)?;
            ctx.set_source_surface(&base_surf, Z, Z)
                .map_err(error_handle)?;
            self.fill_rect(&ctx);
            blur_image_surface(&mut surf, 100)?;
            surf
        };

        {
            ctx.set_source_surface(blur_surface, Z, Z).unwrap();
            self.fill_rect(ctx);

            ctx.set_source_surface(base_surf, Z, Z).unwrap();
            self.fill_rect(ctx);

            // ctx.set_source_surface(&self.predraw.shade, Z, Z).unwrap();
            // self.fill_rect(ctx);

            ctx.set_source_surface(&self.predraw.stroke, Z, Z).unwrap();
            self.fill_rect(ctx);
        }

        self.draw_text(ctx, progress)?;
        Ok(())
    }
    fn draw_text(&self, ctx: &Context, progress: f64) -> Result<(), String> {
        use crate::ui::draws::util::new_surface;
        let (text_surf, text_width) = {
            fn e(e: cairo::Error) -> String {
                format!("Error create surface for text: {e}")
            }
            let surf = new_surface((self.map_size.0, self.map_size.1), e)?;
            let ctx = Context::new(&surf).unwrap();
            let a = get_font_face()?;
            let f_size = self.size.0 * 0.8;
            let y = (self.size.0 - f_size) / 2. + f_size;
            ctx.rotate(-90_f64.to_radians());
            ctx.translate(-self.f_map_size.1, Z);
            ctx.move_to(0., y * 0.9);
            ctx.set_font_face(&a);
            ctx.set_font_size(f_size);
            ctx.set_source_rgb(0., 0., 0.);
            ctx.show_text(format!("{}%", f64::floor(progress * 100.)).as_str())
                .unwrap();
            let w = ctx.current_point().unwrap().0;
            let mut f = File::create("/tmp/test.png").unwrap();
            surf.write_to_png(&mut f).unwrap();
            log::debug!("text size: {}", w);
            (surf, w)
        };

        let (x, y) = match self.edge {
            Edge::Left => {
                if self.is_start {
                    (Z, -self.predraw.slope_position)
                } else {
                    (
                        Z,
                        -(self.f_map_size.1 - self.predraw.slope_position - text_width),
                    )
                }
            }
            Edge::Right => {
                if self.is_start {
                    (Z, -self.predraw.slope_position)
                } else {
                    (
                        Z,
                        -(self.f_map_size.1 - self.predraw.slope_position - text_width),
                    )
                }
            }
            Edge::Top => {
                if self.is_start {
                    (Z, -self.predraw.slope_position)
                } else {
                    (
                        Z,
                        -(self.f_map_size.1 - self.predraw.slope_position - text_width),
                    )
                }
            }
            Edge::Bottom => {
                ctx.rotate(180_f64.to_radians());
                ctx.translate(-self.f_map_size.0, -self.f_map_size.1);
                if self.is_start {
                    (self.extra_trigger_size, -self.predraw.slope_position)
                } else {
                    (
                        self.extra_trigger_size,
                        -(self.f_map_size.1 - self.predraw.slope_position - text_width),
                    )
                }
            }
            _ => unreachable!(),
        };
        ctx.set_source_surface(text_surf, x, y).unwrap();
        self.fill_rect(ctx);
        Ok(())
    }
    fn fill_rect(&self, ctx: &Context) {
        ctx.rectangle(Z, Z, self.f_map_size.0, self.f_map_size.1);
        ctx.fill().unwrap();
    }
    fn new_surface(&self) -> Result<ImageSurface, String> {
        fn e(e: cairo::Error) -> String {
            format!("Draw core new_surface error: {:?}", e)
        }
        new_surface(self.map_size, e)
    }
}
