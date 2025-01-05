use std::{cell::Cell, rc::Rc};

use cairo::ImageSurface;
use config::{Config, MonitorSpecifier};
use gtk::{
    gdk::Monitor,
    prelude::{GtkWindowExt, WidgetExt},
    Application, ApplicationWindow, CssProvider, DrawingArea,
};
use gtk4_layer_shell::LayerShell;

use crate::{
    animation::{AnimationList, ToggleAnimationRc},
    mouse_state::{MouseEvent, MouseStateData},
};

use super::WindowContext;

pub trait WidgetContext {
    fn redraw(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, data: &MouseStateData, event: MouseEvent) -> bool;
}

type PopStateGuard = Rc<()>;

// NOTE: THIS CAN BE MODIFIED ANYTIME WHEN NEEDED
pub struct WindowContextBuilder {
    name: String,
    monitor: MonitorSpecifier,
    window: ApplicationWindow,
    drawing_area: DrawingArea,
    animation_list: AnimationList,

    pop_animation: ToggleAnimationRc,
    pop_state: Rc<Cell<Option<PopStateGuard>>>,
}
impl WindowContextBuilder {
    pub fn new_animation(&mut self, time_cost: u64) -> ToggleAnimationRc {
        self.animation_list.new_transition(time_cost)
    }
    pub fn extend_animation_list(&mut self, list: &AnimationList) {
        self.animation_list.extend_list(list);
    }
    pub fn make_redraw_notifier(&self) -> impl Fn() {
        let drawing_area = &self.drawing_area;
        glib::clone!(
            #[weak]
            drawing_area,
            move || {
                drawing_area.queue_draw();
            }
        )
    }
}

impl WindowContextBuilder {
    /// config and monitor should be ready before this
    pub fn new(app: &Application, monitor: &Monitor, conf: &Config) -> Result<Self, String> {
        let window = gtk::ApplicationWindow::new(app);
        window.set_namespace("way-edges-widget");

        // init layer
        window.init_layer_shell();
        window.set_monitor(monitor);
        window.set_layer(conf.layer);

        // edge and position
        window.set_anchor(conf.edge, true);
        window.set_anchor(conf.position, true);

        if conf.ignore_exclusive {
            window.set_exclusive_zone(-1);
        }

        conf.margins
            .iter()
            .try_for_each(|(e, m)| -> Result<(), String> {
                window.set_margin(*e, m.get_num_into()? as i32);
                Ok(())
            })?;

        // set something after show
        window.connect_show(|w: &gtk::ApplicationWindow| {
            // transparency background !! may not work for some gtk4 theme, and idk what to do with it !!
            let provider = CssProvider::new();
            provider
                // .load_from_string("window.background { background: unset; border: 1px solid white; }");
                .load_from_string("window.background { background: unset; }");
            gtk::style_context_add_provider_for_display(
                &WidgetExt::display(w),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });

        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(1, 1);
        window.set_child(Some(&drawing_area));

        let mut animation_list = AnimationList::new();
        let pop_animation = animation_list.new_transition(conf.transition_duration);

        let pop_state = Rc::new(Cell::new(None));

        Ok(Self {
            name: conf.name.clone(),
            monitor: conf.monitor.clone(),
            window,
            drawing_area,
            animation_list,
            pop_animation,
            pop_state,
        })
    }
    pub fn build(self, widget: impl WidgetContext) -> WindowContext {}
}
