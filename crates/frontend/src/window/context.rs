use std::{any::Any, cell::Cell, rc::Rc};

use super::{
    draw::{make_base_draw_func, make_max_size_func, BaseDrawFunc, MaxSizeFunc},
    event::{WindowPopState, WindowPopStateRc},
    frame::{WindowFrameManager, WindowFrameManagerRc},
};
use config::{Config, MonitorSpecifier};
use gtk::{
    gdk::Monitor,
    prelude::{GtkWindowExt, WidgetExt},
    Application, ApplicationWindow, CssProvider, DrawingArea, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::LayerShell;

use crate::{
    animation::{AnimationList, ToggleAnimationRc},
    buffer::Buffer,
    mouse_state::{self, MouseStateRc},
};

pub struct WindowContext {
    pub name: String,
    pub monitor: MonitorSpecifier,
    pub window: ApplicationWindow,
    pub drawing_area: DrawingArea,
    #[allow(dyn_drop)]
    pub widget_context: Option<Box<dyn Any>>,

    pub(super) frame_manager: WindowFrameManagerRc,

    // draw
    pub(super) image_buffer: Buffer,
    pub(super) max_widget_size_func: MaxSizeFunc,
    pub(super) base_draw_func: BaseDrawFunc,

    // mouse event
    pub(super) start_pos: Rc<Cell<(i32, i32)>>,
    pub(super) mouse_event: MouseStateRc,
    pub window_pop_state: WindowPopStateRc,
}

impl WindowContext {
    /// config and monitor should be ready before this
    pub fn new(app: &Application, monitor: &Monitor, conf: &Config) -> Result<Self, String> {
        let window = gtk::ApplicationWindow::new(app);

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
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });

        window.connect_destroy(|_| {
            log::info!("destroy window");
        });

        window.set_namespace("way-edges-widget");

        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(1, 1);
        window.set_child(Some(&drawing_area));

        let mut animation_list = AnimationList::new();
        let pop_animation = animation_list.new_transition(conf.transition_duration);
        let animation_list = animation_list.make_rc();
        let frame_manager =
            WindowFrameManager::new(conf.frame_rate.unwrap() as u64, animation_list).make_rc();

        // draw
        let extra = conf.extra_trigger_size.get_num_into().unwrap().ceil() as i32;
        let image_buffer = Buffer::default();
        let max_widget_size_func = make_max_size_func(conf.edge, extra);
        let base_draw_func = make_base_draw_func(conf);

        // event
        let start_pos = Rc::new(Cell::new((0, 0)));
        let mouse_event = mouse_state::MouseState::new().connect(&drawing_area);
        let window_pop_state = WindowPopState::new(
            pop_animation,
            Rc::new(glib::clone!(
                #[weak]
                drawing_area,
                move || {
                    drawing_area.queue_draw();
                }
            )),
        )
        .make_rc();

        Ok(Self {
            name: conf.name.clone(),
            monitor: conf.monitor.clone(),
            window,
            drawing_area,
            widget_context: None,

            frame_manager,

            image_buffer,
            max_widget_size_func,
            base_draw_func,

            start_pos,
            mouse_event,
            window_pop_state,
        })
    }
    pub fn show(&self) {
        self.window.present();
    }

    pub fn close(&mut self) {
        self.window.close();
        self.window.destroy();
    }

    pub fn bind_context(&mut self, w: impl Any + 'static) {
        self.widget_context = Some(Box::new(w));
    }

    pub fn new_animation(&self, time_cost: u64) -> ToggleAnimationRc {
        self.frame_manager
            .borrow_mut()
            .animation_list
            .borrow_mut()
            .new_transition(time_cost)
    }

    pub fn extend_animation_list(&self, list: &AnimationList) {
        self.frame_manager
            .borrow_mut()
            .animation_list
            .borrow_mut()
            .extend_list(list);
    }
}

impl Drop for WindowContext {
    fn drop(&mut self) {
        self.close()
    }
}
