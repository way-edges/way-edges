#![cfg(feature = "hyprland")]

mod monitor;
use gio::prelude::ApplicationExt;
use monitor::*;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::activate::{
    calculate_config_relative, create_widgets, get_working_area_size, notify_app_error,
    take_monitor, WidgetItem,
};
use crate::config::GroupConfig;
use gio::glib::idle_add_local_once;
use gtk::gdk::Monitor;
use gtk::glib;
use gtk::prelude::{GtkWindowExt, MonitorExt, WidgetExt};
use gtk::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use scopeguard::defer;

use super::get_monitors;

/// namespace for detect size of available working area
/// TL: Top Left
const NAMESPACE_TL: &str = "way-edges-detect-tl";

/// namespace for detect size of available working area
/// BR: Bottom Right
const NAMESPACE_BR: &str = "way-edges-detect-br";

/// reach max counter
type Counter = Rc<Cell<usize>>;
fn add_or_else(c: &Counter, max: usize) -> bool {
    if c.get() == max - 1 {
        true
    } else {
        c.set(c.get() + 1);
        false
    }
}

/// create window for detection on specific monitor
/// 2 window for positioning: one on top-left corner; one on bottom-right corner
fn window_for_detect(
    app: &Application,
    monitor: &Monitor,
    // layer: Layer,
) -> [ApplicationWindow; 2] {
    // left top
    let win_tl = gtk::ApplicationWindow::new(app);
    win_tl.init_layer_shell();
    // win_tl.set_layer(layer);
    win_tl.set_layer(Layer::Top);
    win_tl.set_anchor(Edge::Top, true);
    win_tl.set_anchor(Edge::Left, true);
    win_tl.set_width_request(1);
    win_tl.set_height_request(1);
    win_tl.set_namespace(NAMESPACE_TL);
    win_tl.set_monitor(monitor);

    // bottom left
    let win_br = gtk::ApplicationWindow::new(app);
    win_br.init_layer_shell();
    // win_br.set_layer(layer);
    win_br.set_layer(Layer::Top);
    win_br.set_anchor(Edge::Bottom, true);
    win_br.set_anchor(Edge::Right, true);
    win_br.set_width_request(1);
    win_br.set_height_request(1);
    win_br.set_namespace(NAMESPACE_BR);
    win_tl.set_monitor(monitor);

    [win_tl, win_br]
}

/// connect realize signal.
/// get layer info from hyprland after rendered.
/// calculate the available area size for each monitor.
/// calculate relative size.
/// render widgets.
fn connect(
    app: &gtk::Application,
    cfgs: GroupConfig,
    needed_monitors: HashMap<String, ()>,
    instance_ref: &Hyprland,
) {
    let windows_count;
    if let Some(vw) = instance_ref.0.take() {
        windows_count = vw.len();
        instance_ref.0.set(Some(vw));
    } else {
        return;
    }

    // as for why so many rc cells, `connect_realize` is not `FnOnce` but `Fn`
    // idk what i can do better for this
    let counter = Rc::new(Cell::new(0));
    let cfgs = Rc::new(Cell::new(cfgs));
    let needed_monitors = Rc::new(Cell::new(needed_monitors));
    let instance = Rc::new(Cell::new(Some(instance_ref.clone())));

    // used to setup realize event for each window
    let connect = gtk::glib::clone!(@weak app, @strong counter, @strong cfgs, @strong needed_monitors, @strong instance => move |w: &ApplicationWindow| {
        w.connect_realize(gtk::glib::clone!(@weak app, @strong counter, @strong cfgs, @strong needed_monitors, @strong instance => move |_| {
            // calculate after all window rendered(windows are not actually rendered when realize signaled)
            idle_add_local_once(
                gtk::glib::clone!(@weak counter, @weak app, @weak cfgs, @weak needed_monitors, @weak instance  => move || {
                    // we need to get all layer info of windows
                    // we are going to do it after the last window rendered
                    // and we use counter to do it
                    if add_or_else(&counter, windows_count) {
                        let (instance, ws) = {
                            let instance = if let Some(instance) = instance.take() {
                                instance
                            }else {
                                // window realized after calculation which is unexpected
                                // it should be closed
                                return;
                            };
                            if let Some(vw) = instance.0.take() {
                                (instance, vw)
                            }else {
                                // position windows are closed meaning application quit
                                return;
                            }
                        };
                        defer!(
                            ws.into_iter().for_each(|w| {
                                w.close();
                            });
                        );
                        // get available area size for all needed monitor
                        let res = get_monitor_map(needed_monitors.take()).and_then(|_| {
                            let cfgs = cfgs.take();
                            let monitors = take_monitor()?;
                            let monitors: HashMap<usize, Monitor> = HashMap::from_iter(monitors.into_iter().enumerate());
                            // create button items
                            let btis = cfgs.into_iter().map(|mut cfg| {
                                // get available area size for each monitor
                                let index = cfg.monitor.to_index()?;
                                let size = get_working_area_size(index)?.ok_or(format!("Did not find Calculated monitor size for {:?}", cfg.monitor))?;
                                calculate_config_relative(&mut cfg, size)?;
                                let monitor = monitors.get(&index).ok_or(format!( "Did not find monitor given index: {index}" ))?.clone();
                                // create widgets
                                Ok(WidgetItem { cfg, monitor })
                            }).collect::<Result<Vec<WidgetItem>, String>>()?;
                            let a = create_widgets(&app, btis)?;
                            instance.0.set(Some(a));
                            Ok(())
                        });
                        if let Err(e) = res {
                            notify_app_error(format!("Failed to initialize app: get_monitor_map(): {e}").as_str());
                            // defer close windows, so we only quit app here
                            idle_add_local_once(glib::clone!(@weak app => move|| {
                                app.quit();
                            }));
                            return;
                        }
                    }
                })
            );
        }));
    });
    unsafe {
        let ovw = instance_ref.0.as_ptr().as_ref().unwrap();
        if let Some(vw) = ovw {
            vw.iter().for_each(|w| {
                connect(w);
            });
        }
    }
}

#[derive(Clone)]
pub struct Hyprland(Rc<Cell<Option<Vec<ApplicationWindow>>>>);
impl super::WindowInitializer for Hyprland {
    fn init_window(app: &Application, cfgs: GroupConfig) -> Result<Self, String> {
        get_monitors().and_then(|monitors| {
            get_need_monitors(&cfgs, monitors).and_then(|ml| {
                // initialize corner windows for eache monitor
                let ws = ml
                    .iter()
                    .flat_map(|m| window_for_detect(app, m))
                    .collect::<Vec<ApplicationWindow>>();

                // all needed monitor's name
                // because hyprland returns monitor's name
                let ml = ml
                    .into_iter()
                    .map(|m| {
                        let name = m
                            .connector()
                            .map(|v| v.to_string())
                            .ok_or(format!("Failed to get monitor name: {m:?}"))?;
                        Ok((name, ()))
                    })
                    .collect::<Result<HashMap<String, ()>, String>>()?;

                let instance = Self(Rc::new(Cell::new(Some(ws.clone()))));
                // setup connect signal
                connect(app, cfgs, ml, &instance);

                // show each window
                ws.iter().for_each(|w| {
                    w.present();
                });
                Ok(instance)
            })
        })
    }
}
impl super::WindowDestroyer for Hyprland {
    fn close_window(self) {
        if let Some(vw) = self.0.take() {
            vw.into_iter().for_each(|w| w.close());
        }
    }
}
