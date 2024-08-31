mod draw;
mod event;

use std::{cell::Cell, rc::Rc, time::Duration};

use draw::DrawCore;
use gio::glib::{clone::Downgrade, WeakRef};
use gtk::{
    glib,
    prelude::{DrawingAreaExtManual, GtkWindowExt, WidgetExt},
    ApplicationWindow, DrawingArea,
};

use crate::{
    config::widgets::hypr_workspace::HyprWorkspaceConfig,
    plug::hypr_workspace::init_hyprland_listener,
    ui::{
        draws::{mouse_state::TranslateStateRc, transition_state::TransitionStateList},
        WidgetExpose, WidgetExposePtr,
    },
};

struct HyprWorkspaceExpose {
    tls: TranslateStateRc,
    darea: WeakRef<DrawingArea>,
}
impl WidgetExpose for HyprWorkspaceExpose {
    fn toggle_pin(&mut self) {
        self.tls.borrow_mut().toggle_pin();
        if let Some(darea) = self.darea.upgrade() {
            darea.queue_draw();
        }
    }
}

pub fn init_widget(
    window: &ApplicationWindow,
    config: crate::config::Config,
    wp_conf: HyprWorkspaceConfig,
) -> Result<WidgetExposePtr, String> {
    println!("Initializing Hyprland Workspace");
    init_hyprland_listener();

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
    let (ms, translate_state) = event::setup_event(&pop_ts, &darea, &workspace_draw_data);

    let mut core = DrawCore::new(
        &darea,
        &config,
        &wp_conf,
        workspace_ts,
        pop_ts.clone(),
        ts_list,
        workspace_draw_data,
    );

    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |_, ctx, _, _| {
            println!("draw");
            core.draw_core(ctx, &window);
        }
    ));

    darea.connect_destroy(move |_| {
        // move lifetime inside destroy
        let _ = &ms;
    });

    Ok(Box::new(HyprWorkspaceExpose {
        tls: translate_state,
        darea: darea.downgrade(),
    }))
}
