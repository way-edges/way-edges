use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use crate::ui::draws::blur::blur_image_surface;
use crate::ui::draws::font::get_font_face;
use crate::ui::draws::frame_manager::FrameManager;
use crate::ui::draws::frame_manager::FrameManagerBindTransition;
use crate::ui::draws::transition_state;
use crate::ui::draws::transition_state::TransitionStateList;
use crate::ui::draws::transition_state::TransitionStateRc;
use crate::ui::draws::util::draw_motion;
use crate::ui::draws::util::draw_rotation;
use crate::ui::draws::util::ensure_input_region;
use crate::ui::draws::util::new_surface;
use crate::ui::draws::util::Z;
use config::widgets::slide::SlideConfig;
use config::Config;

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
use config::widgets::slide::Direction;

pub fn setup_draw(
    window: &gtk::ApplicationWindow,
    cfg: Config,
    mut slide_cfg: SlideConfig,
    mut additional: SlideAdditionalConfig,
) -> Result<SlideExpose, String> {
    let darea = DrawingArea::new();
    let size = slide_cfg.size()?;
    let edge = cfg.edge;
    let direction = slide_cfg.progress_direction;
    let extra_trigger_size = slide_cfg.extra_trigger_size.get_num()?;
    // let f_map_size = (size.0 + extra_trigger_size, size.1);
    let f_map_size = (size.0, size.1);
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

    let mut ts_list = TransitionStateList::new();
    ts_list.extend_list(&std::mem::take(&mut additional.additional_transitions));
    let pop_ts = ts_list
        .new_transition(Duration::from_millis(slide_cfg.transition_duration))
        .item;

    let (progress, ms) = event::setup_event(window, &darea, pop_ts.clone(), &cfg, &mut slide_cfg);

    let predraw = super::pre_draw::draw(
        size,
        map_size,
        slide_cfg.bg_color,
        slide_cfg.border_color,
        slide_cfg.obtuse_angle,
        slide_cfg.radius,
    )?;

    let frame_manager = FrameManager::new(
        slide_cfg.frame_rate,
        glib::clone!(
            #[weak]
            darea,
            move || {
                darea.queue_draw();
            }
        ),
    );

    let mut dc = DrawCore {
        predraw,
        frame_manager,
        ts_list,
        pop_ts,

        progress: progress.clone(),

        edge,
        direction,
        size,
        f_map_size,
        map_size,
        extra_trigger_size,
        is_start: slide_cfg.is_text_position_start,
        text_color: slide_cfg.text_color,

        transition_range,

        fg_color: additional.fg_color,
        additional_callback: additional.on_draw.take(),
    };
    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |_, context, _, _| {
            dc.draw_core(context, &window);
        }
    ));

    darea.connect_destroy(|_| {
        log::debug!("slide drawing area destroyed");
    });

    window.set_child(Some(&darea));
    Ok(SlideExpose {
        darea: Downgrade::downgrade(&darea),
        progress: progress.downgrade(),
        ms: ms.downgrade(),
    })
}

struct DrawCore {
    predraw: SlidePredraw,
    frame_manager: FrameManager,
    ts_list: TransitionStateList,
    pop_ts: TransitionStateRc,

    progress: Rc<Cell<f64>>,

    // normal config
    edge: Edge,
    direction: Direction,
    size: (f64, f64),
    f_map_size: (f64, f64),
    map_size: (i32, i32),
    extra_trigger_size: f64,
    is_start: bool,
    text_color: RGBA,

    transition_range: (f64, f64),

    // additional
    fg_color: Rc<Cell<RGBA>>,
    additional_callback: Option<Box<dyn FnMut()>>,
}
impl DrawCore {
    fn draw_core(&mut self, ctx: &Context, window: &gtk::ApplicationWindow) {
        self.ts_list.refresh();

        if let Some(f) = self.additional_callback.as_mut() {
            f()
        }

        draw_rotation(ctx, self.edge, self.size);
        let y = self.pop_ts.borrow().get_y();
        let visible_y = transition_state::calculate_transition(y, self.transition_range);
        draw_motion(ctx, visible_y, self.transition_range);

        let res = self.draw(ctx, self.progress.get()).map(|_| {
            ensure_input_region(
                window,
                visible_y,
                self.size,
                self.edge,
                self.extra_trigger_size,
            );
            self.frame_manager.ensure_frame_run(&self.ts_list);
        });

        if let Err(e) = res {
            window.close();
            log::error!("{e}");
            util::notify_send("Way-edges widget draw error", &e, true);
        }
    }
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
impl Drop for DrawCore {
    fn drop(&mut self) {
        log::info!("slide draw core dropped");
    }
}
