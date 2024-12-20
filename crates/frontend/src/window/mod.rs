mod draw;

use backend::monitor::get_monitor_context;
use config::{Config, MonitorSpecifier};
use draw::{
    make_motion_func, make_window_input_region_fun, DrawMotionFunc, SetWindowInputRegionFunc,
};
use gtk::{
    gdk::Monitor,
    glib,
    prelude::{DrawingAreaExtManual, GtkWindowExt, MonitorExt, WidgetExt},
    Application, ApplicationWindow, CssProvider, DrawingArea, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::rc::Rc;

use crate::animation::{AnimationList, ToggleAnimationRc};

struct _WindowContext {
    name: String,
    monitor: MonitorSpecifier,
    window: ApplicationWindow,
    drawing_area: DrawingArea,
    pop_animation: ToggleAnimationRc,
    animation_list: AnimationList,

    // func
    motion_func: DrawMotionFunc,
    input_region_func: SetWindowInputRegionFunc,
}

impl _WindowContext {
    /// config and monitor should be ready before this
    fn new(app: &Application, monitor: &Monitor, conf: &Config) -> Result<Self, String> {
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

        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(1, 1);
        window.set_child(Some(&drawing_area));

        let mut animation_list = AnimationList::new();
        let pop_animation = animation_list.new_transition(conf.transition_duration);

        Ok(Self {
            name: conf.name.clone(),
            monitor: conf.monitor.clone(),
            window,
            drawing_area,
            pop_animation,
            animation_list,

            motion_func: make_motion_func(conf.edge, conf.position),
            input_region_func: make_window_input_region_fun(
                conf.edge,
                conf.position,
                conf.extra_trigger_size.get_num_into().unwrap().ceil() as i32,
            ),
        })
    }
    fn show(&self) {
        self.window.present();
    }
}

type RedrawNotifyFunc = Rc<dyn Fn(Option<(i32, i32)>) + 'static>;
impl _WindowContext {
    fn make_redraw_notifier_dyn(&self) -> RedrawNotifyFunc {
        Rc::new(self.make_redraw_notifier())
    }
    fn make_redraw_notifier(&self) -> impl Fn(Option<(i32, i32)>) + 'static {
        let drawing_area = &self.drawing_area;
        let old_size = drawing_area.size_request();
        glib::clone!(
            #[weak]
            drawing_area,
            move |size| {
                if let Some(size) = size {
                    if old_size.0 < size.0 || old_size.1 < size.1 {
                        drawing_area.set_size_request(size.0, size.1);
                    }
                }
                drawing_area.queue_draw();
            }
        )
    }
}
