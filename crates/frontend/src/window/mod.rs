mod draw;

use config::{Config, MonitorSpecifier};
use draw::{make_base_draw_func, make_max_size_func, BaseDrawFunc, Buffer, MaxSizeFunc};
use gtk::{
    gdk::Monitor,
    prelude::{GtkWindowExt, WidgetExt},
    Application, ApplicationWindow, CssProvider, DrawingArea, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::LayerShell;

use crate::animation::{AnimationList, AnimationListRc, ToggleAnimationRc};

struct _WindowContext {
    name: String,
    monitor: MonitorSpecifier,
    window: ApplicationWindow,
    drawing_area: DrawingArea,
    pop_animation: ToggleAnimationRc,
    animation_list: AnimationListRc,

    // draw
    image_buffer: Buffer,
    max_widget_size_func: MaxSizeFunc,
    base_draw_func: BaseDrawFunc,
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
        let animation_list = animation_list.make_rc();

        let extra = conf.extra_trigger_size.get_num_into().unwrap().ceil() as i32;
        Ok(Self {
            name: conf.name.clone(),
            monitor: conf.monitor.clone(),
            window,
            drawing_area,
            pop_animation,
            animation_list,

            image_buffer: Buffer::default(),
            max_widget_size_func: make_max_size_func(conf.edge, extra),
            base_draw_func: make_base_draw_func(conf.edge, conf.position, extra),
        })
    }
    fn show(&self) {
        self.window.present();
    }
}
