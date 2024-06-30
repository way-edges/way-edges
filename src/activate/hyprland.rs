#![cfg(feature = "hyprland")]

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::activate::{calculate_relative, create_buttons, find_monitor, get_monitors, ButtonItem};
use crate::config::{GroupConfig, MonitorSpecifier};
use gio::glib::idle_add_local_once;
use gtk::gdk::Monitor;
use gtk::glib;
use gtk::prelude::{ApplicationExt, GtkWindowExt, WidgetExt};
use gtk::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use hyprland::data::{LayerClient, Layers};
use hyprland::shared::HyprData;
use scopeguard::defer;

const NAMESPACE_TL: &str = "way-edges-detect-tl";
const NAMESPACE_BR: &str = "way-edges-detect-br";
const TOP_LEVEL: &str = "2";

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
        self.1 == self.0.len()
    }
}

type MonitorLayerSizeMap = HashMap<String, (i32, i32)>;
// TODO: only iter included monitors
fn get_monitor_map() -> Result<MonitorLayerSizeMap, String> {
    let mls = Layers::get().map_err(|e| format!("Failed to get layer info: {e}"))?;
    log::debug!("Layer shells from hyprland: {mls:?}");
    let tl_ns = String::from(NAMESPACE_TL);
    let br_ns = String::from(NAMESPACE_BR);
    mls.into_iter()
        .map_while(|(ms, mut d)| {
            let vc = if let Some(v) = d.levels.remove(TOP_LEVEL) {
                v
            } else {
                return Some(Err(format!("No layer info for {ms}")));
            };
            let mut nsm = NameSpaceMatch::new(vec![tl_ns.to_string(), br_ns.to_string()]);
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
                log::debug!("Layer client for monitor({ms}): {lcm:?}");
                // top left
                let tl = lcm.get(&tl_ns.to_string())?;
                let start_x = tl.x;
                let start_y = tl.y;

                // bottom right
                let br = lcm.get(&br_ns.to_string())?;
                let end_x = br.x + br.w as i32;
                let end_y = br.y + br.h as i32;
                // calculate
                let w = end_x - start_x;
                let h = end_y - start_y;

                Some(Ok((ms, (w, h))))
            } else {
                None
            }
        })
        .collect::<Result<HashMap<String, (i32, i32)>, String>>()
}

fn window_for_detect(
    app: &Application,
    monitor: Monitor,
    // layer: Layer,
) -> [Option<ApplicationWindow>; 2] {
    // left top
    let win_tl = gtk::ApplicationWindow::new(app);
    win_tl.init_layer_shell();
    // win_tl.set_layer(layer);
    win_tl.set_layer(Layer::Top);
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
    // win_br.set_layer(layer);
    win_br.set_layer(Layer::Top);
    win_br.set_anchor(Edge::Bottom, true);
    win_br.set_anchor(Edge::Right, true);
    win_br.set_width_request(1);
    win_br.set_height_request(1);
    let brname = String::from("way-edges-detect-br");
    win_br.set_namespace(brname.as_str());
    win_tl.set_monitor(&monitor);

    [Some(win_tl), Some(win_br)]
}

// TODO: Strong ref about ApplicationWindow should be weak
// change Option to WeakRef
fn connect(ws: Vec<Option<ApplicationWindow>>, app: &gtk::Application, cfgs: GroupConfig) {
    // connect show
    let max_count = ws.len();
    let counter = Rc::new(Cell::new(0));
    let cfgs = Rc::new(Cell::new(Some(cfgs)));
    let connect = gtk::glib::clone!(@strong counter, @strong ws, @weak app, @strong cfgs => move |w: &ApplicationWindow| {
        w.connect_realize(gtk::glib::clone!(@strong counter, @strong ws, @weak app, @strong cfgs => move |_| {
            // calculate after all window rendered
            idle_add_local_once(
                gtk::glib::clone!(@weak counter, @strong ws, @weak app, @weak cfgs  => move || {
                    if add_or_else(&counter, max_count) {
                        defer!(
                            ws.into_iter().for_each(|mut w| {
                                if let Some(w) = w.take() {
                                    w.close();
                                }
                            });
                        );
                        let res = get_monitor_map().and_then(|mm| {
                            log::debug!("Calculated layer map sizes: {mm:?}");
                            let monitors = get_monitors()?;
                            let mm = mm.into_iter().map(|(m, s)| {
                                let m = find_monitor(&monitors, MonitorSpecifier::Name(m))?;
                                Ok((m, s))
                            }).collect::<Result<HashMap<Monitor, (i32, i32)>, String>>()?;
                            let cfgs = cfgs.take().ok_or("cfgs is None")?;
                            let btis = cfgs.into_iter().map(|mut cfg| {
                                let monitor = find_monitor(&monitors, cfg.monitor.clone())?;
                                let size = *mm.get(&monitor).ok_or(format!("Did not find Calculated monitor size for {:?}", cfg.monitor))?;
                                calculate_relative(&mut cfg, size)?;
                                Ok(ButtonItem { cfg, monitor })
                            }).collect::<Result<Vec<ButtonItem>, String>>()?;
                            create_buttons(&app, btis);
                            Ok(())
                        });
                        if let Err(e) = res {
                            app.quit();
                            super::notify_app_error(format!("Failed to initialize app: get_monitor_map(): {e}"));
                            return;
                        }
                    }
                })
            );
        }));
    });
    ws.iter().for_each(|w| {
        if let Some(w) = w.as_ref() {
            connect(w);
        } else {
            log::debug!("Positioning window not found")
        }
    });
}

fn get_need_monitors(
    cfgs: &GroupConfig,
    monitors: &gio::ListModel,
) -> Result<Vec<Monitor>, String> {
    let mut mm = HashMap::new();
    cfgs.iter().try_for_each(|cfg| -> Result<(), String> {
        let monitor = super::find_monitor(monitors, cfg.monitor.clone())?;
        mm.entry(monitor).or_insert(());
        Ok(())
    })?;
    Ok(mm.into_keys().collect())
}

pub struct Hyprland;
impl super::WindowInitializer for Hyprland {
    fn init_window(app: &Application, cfgs: GroupConfig) {
        let res = get_monitors().and_then(|monitors| {
            get_need_monitors(&cfgs, &monitors).map(|ml| {
                let ws: Vec<Option<ApplicationWindow>> = ml
                    .into_iter()
                    .flat_map(|m| window_for_detect(app, m))
                    .collect();
                connect(ws.clone(), app, cfgs);

                ws.iter().for_each(|w| {
                    if let Some(w) = w.as_ref() {
                        w.present();
                    } else {
                        log::debug!("Positioning window not found")
                    }
                });
            })
        });
        if let Err(e) = res {
            app.quit();
            super::notify_app_error(e)
        }
    }
}
