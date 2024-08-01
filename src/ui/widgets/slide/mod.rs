mod draw;
mod event;
mod pre_draw;

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use crate::{
    activate::get_monior_size,
    config::{widgets::slide::SlideConfig, Config},
    ui::{
        draws::{mouse_state::TranslateState, transition_state::TransitionStateRc},
        WidgetExpose, WidgetExposePtr,
    },
};
use gio::glib::WeakRef;
use gtk::{gdk::RGBA, ApplicationWindow};

use super::common;

pub struct SlideExpose {
    pub darea: WeakRef<gtk::DrawingArea>,
    pub progress: Weak<Cell<f64>>,
    pub tls: Weak<RefCell<TranslateState>>,
}
impl WidgetExpose for SlideExpose {
    fn toggle_pin(&self) {
        if let Some(tls) = self.tls.upgrade() {
            tls.borrow_mut().toggle_pin();
        }
    }
}

// this is actually for pulseaudio specific, idk how do design this
pub struct SlideAdditionalConfig {
    pub fg_color: Rc<Cell<RGBA>>,
    pub additional_transitions: Vec<TransitionStateRc>,
    pub on_draw: Option<Box<dyn FnMut()>>,
}
impl SlideAdditionalConfig {
    pub fn default(fg_color: RGBA) -> Self {
        Self {
            fg_color: Rc::new(Cell::new(fg_color)),
            additional_transitions: vec![],
            on_draw: None,
        }
    }
}

pub fn init_widget(
    window: &ApplicationWindow,
    config: Config,
    slide_cfg: SlideConfig,
) -> Result<WidgetExposePtr, String> {
    let add = SlideAdditionalConfig {
        fg_color: Rc::new(Cell::new(slide_cfg.fg_color)),
        additional_transitions: vec![],
        on_draw: None,
    };
    let expose = init_widget_as_plug(window, config, slide_cfg, add)?;
    Ok(Box::new(expose))
}

pub fn init_widget_as_plug(
    window: &ApplicationWindow,
    config: Config,
    mut slide_cfg: SlideConfig,
    add: SlideAdditionalConfig,
) -> Result<SlideExpose, String> {
    calculate_rel(&config, &mut slide_cfg)?;
    draw::setup_draw(window, config, slide_cfg, add)
}

fn calculate_rel(config: &Config, slide_config: &mut SlideConfig) -> Result<(), String> {
    let index = config.monitor.to_index()?;
    let size =
        // get_working_area_size(index)?.ok_or(format!("Failed to get working area size: {index}"))?;
        get_monior_size(index)?.ok_or(format!("Failed to get working area size: {index}"))?;

    common::calculate_rel_extra_trigger_size(
        &mut slide_config.extra_trigger_size,
        size,
        config.edge,
    );

    common::calculate_rel_width_height(
        &mut slide_config.width,
        &mut slide_config.height,
        size,
        config.edge,
    )?;
    Ok(())
}
