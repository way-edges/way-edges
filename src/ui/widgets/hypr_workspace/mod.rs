mod draw;
mod event;

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

use draw::DrawCore;
use gio::glib::clone::Downgrade;
use gtk::{
    glib,
    prelude::{DrawingAreaExtManual, GtkWindowExt, WidgetExt},
    ApplicationWindow, DrawingArea,
};

use crate::{
    activate::get_monior_size,
    config::widgets::hypr_workspace::HyprWorkspaceConfig,
    plug::hypr_workspace::init_hyprland_listener,
    ui::{
        draws::{mouse_state::MouseState, transition_state::TransitionStateList},
        WidgetExpose, WidgetExposePtr,
    },
};

struct HyprWorkspaceExpose {
    ms: Weak<RefCell<MouseState>>,
    // darea: WeakRef<DrawingArea>,
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
    let monitor = config.monitor.to_index()?;

    let size = get_monior_size(monitor)?.ok_or(format!("Failed to get monitor size: {monitor}"))?;

    match config.edge {
        gtk4_layer_shell::Edge::Left | gtk4_layer_shell::Edge::Right => {
            wp_conf.thickness.calculate_relative(size.0 as f64);
            wp_conf.length.calculate_relative(size.1 as f64);
            wp_conf.extra_trigger_size.calculate_relative(size.0 as f64);
        }
        gtk4_layer_shell::Edge::Top | gtk4_layer_shell::Edge::Bottom => {
            wp_conf.thickness.calculate_relative(size.1 as f64);
            wp_conf.length.calculate_relative(size.0 as f64);
            wp_conf.extra_trigger_size.calculate_relative(size.1 as f64);
        }
        _ => unreachable!(),
    }

    Ok(())
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
    match config.edge {
        gtk4_layer_shell::Edge::Left | gtk4_layer_shell::Edge::Right => {
            darea.set_size_request(
                wp_conf.thickness.get_num().unwrap().ceil() as i32,
                wp_conf.length.get_num().unwrap().ceil() as i32,
            );
        }
        gtk4_layer_shell::Edge::Top | gtk4_layer_shell::Edge::Bottom => {
            darea.set_size_request(
                wp_conf.length.get_num().unwrap().ceil() as i32,
                wp_conf.thickness.get_num().unwrap().ceil() as i32,
            );
        }
        _ => todo!(),
    };

    let mut ts_list = TransitionStateList::new();
    let pop_ts = ts_list
        .new_transition(Duration::from_millis(wp_conf.transition_duration))
        .item;

    let workspace_ts = ts_list
        .new_transition(Duration::from_millis(wp_conf.workspace_transition_duration))
        .item;

    let workspace_draw_data = Rc::new(Cell::new(draw::DrawData::new(config.edge)));
    let hover_id = Rc::new(Cell::new(-1));
    let ms = event::setup_event(&pop_ts, &darea, &workspace_draw_data, &hover_id);

    let mut core = DrawCore::new(
        &darea,
        &config,
        &wp_conf,
        workspace_ts,
        pop_ts.clone(),
        ts_list,
        workspace_draw_data,
        hover_id,
        &ms,
    );

    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |_, ctx, _, _| {
            core.draw_core(ctx, &window);
        }
    ));

    Ok(Box::new(HyprWorkspaceExpose {
        ms: ms.downgrade(),
        // darea: darea.downgrade(),
    }))
}
