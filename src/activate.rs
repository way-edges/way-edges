use crate::config::{GroupConfig, MonitorSpecifier};
use gio::prelude::*;
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use gtk::Application;

fn get_monitors() -> gio::ListModel {
    let dt_display = gtk::gdk::Display::default().expect("display for monitor not found");
    dt_display.monitors()
}

fn find_monitor(monitors: &gio::ListModel, specifier: MonitorSpecifier) -> Monitor {
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
    fn init_window(app: &Application, cfgs: GroupConfig);
}

// !!!not finished yet
#[cfg(feature = "hyprland")]
pub mod compositor_hyprland {
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::config::GroupConfig;
    use gio::glib::idle_add_local_once;
    use gtk::gdk::Monitor;
    use gtk::glib;
    use gtk::prelude::{GtkWindowExt, WidgetExt};
    use gtk::{Application, ApplicationWindow};
    use gtk4_layer_shell::{Edge, Layer, LayerShell};
    use hyprland::data::{LayerClient, Layers};
    use hyprland::shared::HyprData;

    const NAMESPACE_TL: &str = "way-edges-detect-tl";
    const NAMESPACE_BR: &str = "way-edges-detect-br";

    type Counter = Rc<Cell<usize>>;
    fn add_or_else(c: &Counter, max: usize) -> bool {
        if c.get() == max - 1 {
            true
        } else {
            c.set(c.get() + 1);
            false
        }
    }

    struct NameSpaceMatch(HashMap<String, bool>, usize);
    impl NameSpaceMatch {
        fn new(vs: Vec<String>) -> Self {
            NameSpaceMatch(HashMap::from_iter(vs.into_iter().map(|s| (s, false))), 0)
        }
        fn ok(&mut self, s: &String) -> bool {
            if let Some(b) = self.0.get(s) {
                if *b {
                    panic!("{s} found twice");
                } else {
                    self.0.insert(s.clone(), true);
                    self.1 += 1;
                    true
                }
            } else {
                false
            }
        }
        fn is_finish(&self) -> bool {
            println!("len: {} | 1: {}", self.0.len(), self.1);
            self.1 == self.0.len()
        }
    }

    type MonitorLevelLayerSizeMap = HashMap<String, HashMap<Layer, (i32, i32)>>;
    fn get_layer_map() -> MonitorLevelLayerSizeMap {
        let ls = Layers::get().unwrap();
        println!("layer shell: {ls:#?}");
        let tl_ns = String::from(NAMESPACE_TL);
        let br_ns = String::from(NAMESPACE_BR);
        let res = ls
            .into_iter()
            .map(|(ms, d)| {
                let lvs = d
                    .levels
                    .into_iter()
                    .filter_map(|(l, vc)| {
                        let mut nsm =
                            NameSpaceMatch::new(vec![tl_ns.to_string(), br_ns.to_string()]);
                        let lcm = vc
                            .into_iter()
                            .filter_map(|c| {
                                if nsm.ok(&c.namespace) {
                                    Some((c.namespace.clone(), c))
                                } else {
                                    None
                                }
                            })
                            .collect::<HashMap<String, LayerClient>>();
                        if nsm.is_finish() {
                            println!("layer client: {lcm:#?}");
                            // top left
                            let tl = lcm.get(&tl_ns.to_string()).unwrap();
                            let start_x = tl.x;
                            let start_y = tl.y;

                            // bottom right
                            let br = lcm.get(&br_ns.to_string()).unwrap();
                            let end_x = br.x + br.w as i32;
                            let end_y = br.y + br.h as i32;
                            // calculate
                            let w = end_x - start_x;
                            let h = end_y - start_y;

                            // layer
                            let layer = match l.as_str() {
                                "0" => Layer::Background,
                                "1" => Layer::Bottom,
                                "2" => Layer::Top,
                                "3" => Layer::Overlay,
                                _ => unreachable!(),
                            };

                            Some((layer, (w, h)))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<Layer, (i32, i32)>>();
                // println!("layer map: {lvs:#?}");
                (ms, lvs)
            })
            .collect::<HashMap<String, HashMap<Layer, (i32, i32)>>>();
        res
    }

    fn window_for_detect(
        app: &Application,
        monitor: Monitor,
        layer: Layer,
    ) -> [Option<ApplicationWindow>; 2] {
        // left top
        let win_tl = gtk::ApplicationWindow::new(app);
        win_tl.init_layer_shell();
        win_tl.set_layer(layer);
        win_tl.set_anchor(Edge::Top, true);
        win_tl.set_anchor(Edge::Left, true);
        win_tl.set_width_request(1);
        win_tl.set_height_request(1);
        let tlname = String::from("way-edges-detect-tl");
        win_tl.set_namespace(tlname.as_str());
        win_tl.set_monitor(&monitor);

        // bottom left
        let win_br = gtk::ApplicationWindow::new(app);
        win_br.init_layer_shell();
        win_br.set_layer(layer);
        win_br.set_anchor(Edge::Bottom, true);
        win_br.set_anchor(Edge::Right, true);
        win_br.set_width_request(1);
        win_br.set_height_request(1);
        let brname = String::from("way-edges-detect-br");
        win_br.set_namespace(brname.as_str());
        win_tl.set_monitor(&monitor);

        [Some(win_tl), Some(win_br)]
    }

    fn connect(ws: Vec<Option<ApplicationWindow>>, app: &gtk::Application) {
        // connect show
        let max_count = ws.len();
        let counter = Rc::new(Cell::new(0));
        let connect = gtk::glib::clone!(@strong counter, @strong ws, @weak app => move |w: &ApplicationWindow| {
            // `connect_realize` only accept `Fn`
            // only way i can think of is wrap function with rc
            // glib::clone! macro is also `Fn` not `FnOnce`
            w.connect_realize(gtk::glib::clone!(@strong counter, @strong ws, @weak app => move |_| {
                idle_add_local_once(
                    gtk::glib::clone!(@strong counter, @strong ws, @weak app  => move || {
                        if add_or_else(&counter, max_count) {
                            let a = get_layer_map();
                            ws.into_iter().for_each(|mut w| {
                                w.take().unwrap().close();
                            });
                            println!("layer map: {a:#?}");
                        }
                    })
                );
            }));
        });
        ws.iter().for_each(|w| {
            connect(&w.clone().unwrap());
        });
    }

    fn get_monitor_layer_map(
        cfgs: &GroupConfig,
        monitors: &gio::ListModel,
    ) -> Vec<(Monitor, Layer)> {
        let mut mm = HashMap::new();
        cfgs.iter().for_each(|cfg| {
            let monitor = super::find_monitor(monitors, cfg.monitor.clone());
            let lm = mm.entry(monitor).or_insert(HashMap::new());
            lm.entry(cfg.layer).or_insert(());
        });
        let a: Vec<(Monitor, Layer)> = mm
            .into_iter()
            .flat_map(|(m, lm)| {
                let layers: Vec<(Monitor, Layer)> =
                    lm.into_keys().map(|l| (m.clone(), l)).collect();
                layers
            })
            .collect();
        a
    }

    pub struct Hyprland;
    impl super::WindowInitializer for Hyprland {
        fn init_window(app: &Application, cfgs: GroupConfig) {
            let monitors = super::get_monitors();
            let mlm = get_monitor_layer_map(&cfgs, &monitors);
            println!("monitor layer map: {mlm:#?}");
            let ws: Vec<Option<ApplicationWindow>> = mlm
                .into_iter()
                .flat_map(|(m, l)| window_for_detect(app, m, l))
                .collect();
            println!("into connect");
            connect(ws.clone(), app);
            ws.iter().for_each(|f| {
                f.as_ref().unwrap().present();
            });
        }
    }
}

pub mod compositor_default {
    use super::find_monitor;
    use crate::config::{Config, GroupConfig};
    use crate::ui;
    use gtk::gdk::{Monitor, Rectangle};
    use gtk::prelude::MonitorExt;
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

    pub struct ButtonItem {
        cfg: Config,
        monitor: Monitor,
    }

    pub fn create_buttons(app: &gtk::Application, button_items: Vec<ButtonItem>) {
        button_items.into_iter().for_each(|bti| {
            // match relative height
            let window = ui::new_window(app, bti.cfg);
            window.set_monitor(&bti.monitor);
            window.set_namespace("way-edges-widget");
        });
    }

    pub struct Default;
    impl super::WindowInitializer for Default {
        fn init_window(app: &gtk::Application, cfgs: GroupConfig) {
            let monitors = super::get_monitors();
            let btis: Vec<ButtonItem> = cfgs
                .into_iter()
                .map(|mut cfg| {
                    let monitor = find_monitor(&monitors, cfg.monitor.clone());
                    if cfg.rel_height > 0. {
                        calculate_height(&mut cfg, monitor.geometry());
                    };
                    ButtonItem { cfg, monitor }
                })
                .collect();
            create_buttons(app, btis);
        }
    }
}
