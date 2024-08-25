use std::{
    cell::Cell,
    rc::Rc,
    sync::{atomic::AtomicPtr, Arc},
};

use cairo::{Context, Format, ImageSurface, LinearGradient};
use gtk::{gdk::RGBA, ApplicationWindow};
use gtk4_layer_shell::Edge;

use crate::{
    config::widgets::hypr_workspace::HyprWorkspaceConfig,
    plug::hypr_workspace::{
        init_hyprland_listener, register_hypr_event_callback, unregister_hypr_event_callback,
        HyprGlobalData,
    },
    ui::{
        draws::{
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

    Ok(Box::new(HyprWorkspaceExpose))
}

type MaxWorkspace = i32;
type WorkspaceID = i32;

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
        use gtk::glib;
        let (id, init_data) = register_hypr_event_callback(glib::clone!(
            #[weak]
            data,
            move |f| {
                data.set(f.clone());
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
        let data = self.data.as_ref().as_ptr();
        let item_base_length = {
            let up = (self.length - self.gap * (self.max_workspace - 1)) as f64;
            up / self.max_workspace as f64
        };
        let item_changable_length = item_base_length * self.active_increase;
        println!("{item_base_length}, {item_changable_length}");

        let item_max_length = item_base_length + item_changable_length;
        let item_min_length =
            item_base_length - item_changable_length / (self.max_workspace - 1) as f64;
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

        let y = self
            .workspace_transition
            .get_abs(self.workspace_transition.y);

        println!("{y}");

        let a: Vec<(f64, RGBA)> = (1..=self.max_workspace)
            .map(|w| {
                if w == self.current_workspace {
                    (
                        item_min_length + (item_max_length - item_min_length) * y,
                        color_transition(self.deactive_color, self.active_color, y),
                    )
                } else if w == self.last_workspace {
                    (
                        item_min_length + (item_max_length - item_min_length) * (1. - y),
                        color_transition(self.active_color, self.deactive_color, y),
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
