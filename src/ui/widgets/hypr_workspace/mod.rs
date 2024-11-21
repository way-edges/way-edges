mod draw;
mod event;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::Duration,
};

use draw::DrawCore;
use gio::glib::clone::Downgrade;
use gtk::{
    glib,
    prelude::{DrawingAreaExtManual, GtkWindowExt},
    ApplicationWindow, DrawingArea,
};

use crate::{
    activate::monitor::get_monitor_context,
    config::widgets::hypr_workspace::HyprWorkspaceConfig,
    plug::hypr_workspace::init_hyprland_listener,
    ui::{
        draws::{mouse_state::MouseState, transition_state::TransitionStateList},
        WidgetExpose, WidgetExposePtr,
    },
};

use super::common::calculate_rel_extra_trigger_size;

struct HyprWorkspaceExpose {
    ms: Weak<RefCell<MouseState>>,
}
impl WidgetExpose for HyprWorkspaceExpose {
    fn toggle_pin(&mut self) {
        if let Some(ms) = self.ms.upgrade() {
            ms.borrow_mut().toggle_pin();
        }
    }
}

fn calculate_raletive(
    config: &crate::config::Config,
    wp_conf: &mut HyprWorkspaceConfig,
) -> Result<(), String> {
    let size = get_monitor_context()
        .get_monitor_size(&config.monitor)
        .ok_or(format!("Failed to get monitor size: {:?}", config.monitor))?;

    calculate_rel_extra_trigger_size(&mut wp_conf.extra_trigger_size, size, config.edge);

    wp_conf.size.ensure_no_relative(size, config.edge)
}

pub fn init_widget(
    window: &ApplicationWindow,
    config: crate::config::Config,
    mut wp_conf: HyprWorkspaceConfig,
) -> Result<WidgetExposePtr, String> {
    init_hyprland_listener();

    calculate_raletive(&config, &mut wp_conf)?;

    let darea = DrawingArea::new();
    window.set_child(Some(&darea));

    let mut ts_list = TransitionStateList::new();
    let pop_ts = ts_list
        .new_transition(Duration::from_millis(wp_conf.transition_duration))
        .item;

    let workspace_ts = ts_list
        .new_transition(Duration::from_millis(wp_conf.workspace_transition_duration))
        .item;

    let hover_data = Rc::new(RefCell::new(draw::HoverData::new(config.edge)));
    let ms = event::setup_event(&pop_ts, &darea, &hover_data);

    let mut core = DrawCore::new(
        &darea,
        &config,
        &wp_conf,
        workspace_ts,
        pop_ts.clone(),
        ts_list,
        hover_data,
        &ms,
    );

    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |_, ctx, _, _| {
            core.draw_core(ctx, &window);
        }
    ));

    Ok(Box::new(HyprWorkspaceExpose { ms: ms.downgrade() }))
}
