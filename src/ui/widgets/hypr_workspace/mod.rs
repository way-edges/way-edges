use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use cairo::{Context, Format, ImageSurface, LinearGradient};
use gtk::{
    gdk::RGBA,
    glib,
    prelude::{DrawingAreaExtManual, GdkCairoContextExt},
    ApplicationWindow, DrawingArea,
};

use crate::{
    config::widgets::hypr_workspace::HyprWorkspaceConfig,
    plug::hypr_workspace::{
        init_hyprland_listener, register_hypr_event_callback, unregister_hypr_event_callback,
        HyprGlobalData,
    },
    ui::{
        draws::{
            mouse_state::{new_mouse_state, new_translate_mouse_state},
            transition_state::{TransitionState, TransitionStateRc},
            util::{color_transition, Z},
        },
        WidgetExpose, WidgetExposePtr,
    },
};

struct HyprWorkspaceExpose;

impl WidgetExpose for HyprWorkspaceExpose {
    fn close(&mut self) {}
    fn toggle_pin(&mut self) {}
}

pub fn init_widget(
    window: &ApplicationWindow,
    config: crate::config::Config,
    wp_conf: HyprWorkspaceConfig,
) -> Result<WidgetExposePtr, String> {
    init_hyprland_listener();

    let darea = DrawingArea::new();

    let mouse_state = new_mouse_state(&darea);
    let pop_transition_state = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        wp_conf.transition_duration,
    ))));
    let (cb, translate_state) = new_translate_mouse_state(
        pop_transition_state.clone(),
        mouse_state.clone(),
        None,
        false,
    );
    mouse_state.borrow_mut().set_event_cb(cb);

    let workspace_transition = Rc::new(RefCell::new(TransitionState::new(Duration::from_millis(
        wp_conf.workspace_transition_duration,
    ))));
    let transition_list = [pop_transition_state, workspace_transition.clone()];

    let core = DrawCore::new(&wp_conf, workspace_transition);
    darea.set_draw_func(|_, ctx, _, _| {});

    Ok(Box::new(HyprWorkspaceExpose))
}

struct DrawCore {
    data: Rc<Cell<HyprGlobalData>>,

    thickness: i32,
    length: i32,
    gap: i32,
    active_increase: f64,

    backlight: Option<RGBA>,
    deactive_color: RGBA,
    active_color: RGBA,

    workspace_transition: TransitionStateRc,

    // for lifetime usage:
    hypr_event_callback_id: u32,
}

impl Drop for DrawCore {
    fn drop(&mut self) {
        unregister_hypr_event_callback(self.hypr_event_callback_id)
    }
}

impl DrawCore {
    fn new(wp_conf: &HyprWorkspaceConfig, workspace_transition: TransitionStateRc) -> Self {
        let data = Rc::new(Cell::new(HyprGlobalData::default()));
        let (id, init_data) = register_hypr_event_callback(glib::clone!(
            #[weak]
            data,
            move |f| {
                data.set(*f);
            }
        ));
        data.set(init_data);
        Self {
            data,

            thickness: 10,
            length: 200,
            gap: 5,
            active_increase: 0.5,

            backlight: Some(RGBA::BLUE),
            deactive_color: RGBA::BLACK,
            active_color: RGBA::BLUE,

            workspace_transition,

            hypr_event_callback_id: id,
        }
    }

    fn draw(&self) -> ImageSurface {
        let data = self.data.get();
        let item_base_length = {
            let up = (self.length - self.gap * (data.max_workspace - 1)) as f64;
            up / data.max_workspace as f64
        };
        let item_changable_length = item_base_length * self.active_increase;
        println!("{item_base_length}, {item_changable_length}");

        let item_max_length = item_base_length + item_changable_length;
        let item_min_length =
            item_base_length - item_changable_length / (data.max_workspace - 1) as f64;
        println!("{item_max_length}, {item_min_length}");

        let surf = ImageSurface::create(Format::ARgb32, self.thickness, self.length).unwrap();
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

        println!("{y}");

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

        println!("{a:#?}");

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
