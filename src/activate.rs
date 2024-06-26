use crate::config::{Config, MonitorSpecifier};
use gio::prelude::*;
use gtk::gdk::Monitor;
use gtk::prelude::MonitorExt;
use gtk::Application;

pub fn find_monitor(monitors: &gio::ListModel, specifier: MonitorSpecifier) -> Monitor {
    let a = match specifier {
        MonitorSpecifier::ID(index) => {
            let a = monitors
                .iter::<Monitor>()
                .nth(index)
                .unwrap_or_else(|| panic!("error matching monitor with id: {index}"))
                .unwrap_or_else(|_| panic!("error matching monitor with id: {index}"));
            a
        }
        MonitorSpecifier::Name(name) => {
            for m in monitors.iter() {
                let m: Monitor = m.unwrap();
                if m.connector().unwrap() == name {
                    return m;
                }
            }
            panic!("monitor with name: {name} not found");
        }
    };
    a
}

pub trait WindowInitializer {
    fn init_window(app: &Application, cfgs: Vec<Config>);
}

// !!!not finished yet
#[cfg(feature = "hyprland")]
pub mod compositor_hyprland {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::config::{Config, MonitorSpecifier};
    use crate::ui;
    use gio::prelude::*;
    use gtk::gdk::Monitor;
    use gtk::prelude::{DisplayExt, GtkWindowExt, MonitorExt, WidgetExt};
    use gtk::Application;
    use gtk4_layer_shell::{Edge, Layer, LayerShell};

    fn init_window() {}

    struct Counter(u8);
    impl Counter {
        fn add(&mut self) -> bool {
            if self.0 == 1 {
                true
            } else {
                self.0 += 1;
                false
            }
        }
    }

    fn window_for_detect(app: &Application) {
        let counter = Rc::new(Cell::new(0));
        // left top
        let window = gtk::ApplicationWindow::new(app);
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_width_request(1);
        window.set_height_request(1);
        let tlname = "way-edges-detect-tl";
        window.set_namespace(tlname);
        window.present();
        window.connect_show(|w| {});

        // left top
        let window = gtk::ApplicationWindow::new(app);
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Right, true);
        window.set_width_request(1);
        window.set_height_request(1);
        let brname = "way-edges-detect-br";
        window.set_namespace(brname);
        window.present();
    }

    pub struct Hyprland;
    impl super::WindowInitializer for Hyprland {
        fn init_window(app: &Application, cfgs: Vec<Config>) {
            window_for_detect(app);
        }
    }
}

pub mod compositor_unknow {
    use super::find_monitor;
    use crate::config::{Config, GroupConfig};
    use crate::ui;
    use gtk::gdk::Rectangle;
    use gtk::prelude::{DisplayExt, MonitorExt};
    use gtk4_layer_shell::{Edge, LayerShell};

    fn calculate_height(cfg: &mut Config, geom: Rectangle) {
        let mx = match cfg.edge {
            Edge::Left | Edge::Right => geom.height(),
            Edge::Top | Edge::Bottom => geom.width(),
            _ => unreachable!(),
        };
        cfg.size.1 = mx as f64 * cfg.rel_height;
        // remember to check height since we didn't do it in `parse_config`
        // when passing only `rel_height`
        if cfg.size.0 * 2. > cfg.size.1 {
            panic!("relative height detect: width * 2 must be <= height: {{cfg.size.0}} * 2 <= {{cfg.size.1}}");
        }
    }

    pub struct Unknow;
    impl super::WindowInitializer for Unknow {
        fn init_window(app: &gtk::Application, cfgs: GroupConfig) {
            let dt_display = gtk::gdk::Display::default().expect("display for monitor not found");
            cfgs.into_iter().for_each(|mut cfg| {
                let monitor = find_monitor(&dt_display.monitors(), cfg.monitor.clone());
                // match relative height
                if cfg.rel_height > 0. {
                    calculate_height(&mut cfg, monitor.geometry());
                };
                let window = ui::new_window(app, cfg);
                window.set_monitor(&monitor);
                window.set_namespace("way-edges-widget");
            });
        }
    }
}
