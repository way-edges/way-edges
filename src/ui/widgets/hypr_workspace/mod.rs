mod draw;

use std::time::Duration;

use draw::DrawCore;
use gtk::{prelude::DrawingAreaExtManual, prelude::GtkWindowExt, ApplicationWindow, DrawingArea};

use crate::{
    config::widgets::hypr_workspace::HyprWorkspaceConfig,
    plug::hypr_workspace::init_hyprland_listener,
    ui::{
        draws::{
            mouse_state::{new_mouse_state, new_translate_mouse_state},
            transition_state::TransitionStateList,
        },
        WidgetExpose, WidgetExposePtr,
    },
};

struct HyprWorkspaceExpose;
impl WidgetExpose for HyprWorkspaceExpose {}

pub fn init_widget(
    window: &ApplicationWindow,
    config: crate::config::Config,
    wp_conf: HyprWorkspaceConfig,
) -> Result<WidgetExposePtr, String> {
    println!("Initializing Hyprland Workspace");
    init_hyprland_listener();

    let darea = DrawingArea::new();
    window.set_child(Some(&darea));

    let mouse_state = new_mouse_state(&darea);

    let mut ts_list = TransitionStateList::new();
    let pop_ts = ts_list.new_transition(Duration::from_millis(wp_conf.transition_duration));

    let workspace_ts =
        ts_list.new_transition(Duration::from_millis(wp_conf.workspace_transition_duration));

    let (cb, translate_state) =
        new_translate_mouse_state(pop_ts.clone(), mouse_state.clone(), None, false);
    mouse_state.borrow_mut().set_event_cb(cb);

    let mut core = DrawCore::new(
        &darea,
        &config,
        &wp_conf,
        workspace_ts,
        pop_ts.clone(),
        ts_list,
    );
    use gtk::glib;
    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |_, ctx, _, _| {
            println!("draw");
            core.draw_core(ctx, &window);
        }
    ));

    Ok(Box::new(HyprWorkspaceExpose))
}
