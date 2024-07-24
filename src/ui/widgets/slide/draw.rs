use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::time::Duration;

use crate::config::widgets::slide::SlideConfig;
use crate::config::Config;
use crate::ui::draws::blur::blur_image_surface;
use crate::ui::draws::font::get_font_face;
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::transition_state;
use crate::ui::draws::transition_state::is_in_transition;
use crate::ui::draws::transition_state::TransitionState;
use crate::ui::draws::transition_state::TransitionStateRc;
use crate::ui::draws::util::draw_input_region_now;
use crate::ui::draws::util::draw_motion_now;
use crate::ui::draws::util::draw_rotation_now;
use crate::ui::draws::util::new_surface;
use crate::ui::draws::util::Z;

use cairo::ImageSurface;
use gio::glib::clone::Downgrade;
use gtk::cairo;
use gtk::cairo::Context;
use gtk::gdk::RGBA;
use gtk::glib;
use gtk::prelude::*;
use gtk::DrawingArea;
use gtk4_layer_shell::Edge;

use super::event;
use super::pre_draw::SlidePredraw;
use super::SlideAdditionalConfig;
use super::SlideExpose;
use crate::config::widgets::slide::Direction;

pub fn setup_draw(
    window: &gtk::ApplicationWindow,
    cfg: Config,
    mut slide_cfg: SlideConfig,
    mut additional: SlideAdditionalConfig,
) -> Result<SlideExpose, String> {
    let darea = DrawingArea::new();
    let size = slide_cfg.get_size()?;
    let edge = cfg.edge;
    let direction = slide_cfg.progress_direction;
    let extra_trigger_size = slide_cfg.extra_trigger_size.get_num()?;
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

    let transition_range = (slide_cfg.preview_size, size.0);
    println!("transition_range: {transition_range:?}");
    let ts = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        slide_cfg.transition_duration,
    ))));
    let progress = event::setup_event(&darea, ts.clone(), &cfg, &mut slide_cfg);

    let predraw = super::pre_draw::draw(
        size,
        map_size,
        slide_cfg.bg_color,
        slide_cfg.border_color,
        slide_cfg.obtuse_angle,
        slide_cfg.radius,
    )?;
    let dc = DrawCore {
        predraw,
        edge,
        direction,
        size,
        f_map_size,
        map_size,
        extra_trigger_size,
        is_start: slide_cfg.is_text_position_start,
        text_color: slide_cfg.text_color,
        fg_color: additional.fg_color,
    };
    let mut frame_manager = FrameManager::new(slide_cfg.frame_rate, &darea, window);
    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        #[strong]
        progress,
        move |_, context, _, _| {
            if let Some(f) = additional.on_draw.as_mut() {
                f()
            }
            draw_rotation_now(context, dc.edge, dc.size);
            let y = ts.borrow().get_y();
            let visible_y = transition_state::calculate_transition(y, transition_range);
            println!("{ts:?}");
            draw_motion_now(
                context,
                visible_y,
                dc.edge,
                transition_range,
                dc.extra_trigger_size,
            );

            let res = dc.draw(context, progress.get()).and_then(|_| {
                draw_input_region_now(&window, visible_y, dc.size, dc.edge, dc.extra_trigger_size)
                    .and_then(|_| {
                        draw_frame_manager_multiple_transition(
                            &mut frame_manager,
                            &additional.additional_transitions,
                            y,
                        )
                    })
            });

            if let Err(e) = res {
                window.close();
                log::error!("{e}");
                crate::notify_send("Way-edges widget draw error", &e, true);
            }
        }
    ));
    window.set_child(Some(&darea));
    Ok(SlideExpose {
        darea: Downgrade::downgrade(&darea),
        progress: progress.downgrade(),
    })
}

pub fn draw_frame_manager_multiple_transition(
    frame_manager: &mut FrameManager,
    tss: &[TransitionStateRc],
    visible_y: f64,
) -> Result<(), String> {
    if is_in_transition(visible_y) || tss.iter().any(|f| f.borrow().is_in_transition()) {
        frame_manager.start()?;
    } else {
        frame_manager.stop()?;
    }
    Ok(())
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
    text_color: RGBA,
    fg_color: Rc<Cell<RGBA>>,
}
impl DrawCore {
    fn draw(&self, ctx: &Context, progress: f64) -> Result<(), String> {
        fn error_handle(e: cairo::Error) -> String {
            format!("Draw core error: {:?}", e)
        }
        let base_surf = {
            let surf = self.new_surface();
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
                let fg_surf = {
                    let surf = self.new_surface();
                    let ctx = cairo::Context::new(&surf).map_err(error_handle)?;
                    ctx.set_source_color(&self.fg_color.get());
                    ctx.append_path(&self.predraw.path);
                    ctx.fill().map_err(error_handle)?;
                    surf
                };
                ctx.set_source_surface(fg_surf, Z, (progress - 1.) * self.size.1)
                    .unwrap();
                ctx.append_path(&self.predraw.path);
                ctx.fill().map_err(error_handle)?;
            };
            surf
        };

        let blur_surface = {
            let mut surf = self.new_surface();
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
        let (text_surf, text_width) = {
            let surf = self.new_surface();
            let ctx = Context::new(&surf).unwrap();
            let a = get_font_face()?;
            let f_size = self.size.0 * 0.8;
            let y = (self.size.0 - f_size) / 2. + f_size;
            ctx.rotate(-90_f64.to_radians());
            ctx.translate(-self.f_map_size.1, Z);
            ctx.move_to(0., y * 0.9);
            ctx.set_font_face(&a);
            ctx.set_font_size(f_size);
            ctx.set_source_color(&self.text_color);
            ctx.show_text(format!("{}%", f64::floor(progress * 100.)).as_str())
                .unwrap();
            let w = ctx.current_point().unwrap().0;
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
    fn new_surface(&self) -> ImageSurface {
        new_surface(self.map_size)
    }
}
