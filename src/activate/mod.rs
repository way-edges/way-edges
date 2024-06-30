pub mod default;
pub mod hyprland;

use crate::config::{Config, GroupConfig, MonitorSpecifier, NumOrRelative};
use crate::ui;
use gio::prelude::*;
use gtk::gdk::Monitor;
use gtk::prelude::{DisplayExt, MonitorExt};
use gtk::Application;
use gtk4_layer_shell::{Edge, LayerShell};

fn notify_app_error(err_des: String) {
    log::error!("{err_des}");
    if let Err(e) = notify_rust::Notification::new()
        .summary("Way-edges")
        .body(&err_des)
        .urgency(notify_rust::Urgency::Critical)
        .show()
    {
        log::error!("error sending Error notification: {e}");
    }
}

fn get_monitors() -> Result<gio::ListModel, String> {
    let dt_display = gtk::gdk::Display::default().ok_or("display for monitor not found")?;
    let mms = dt_display.monitors();
    log::debug!("Get monitors: {mms:?}");
    Ok(mms)
}

fn find_monitor(monitors: &gio::ListModel, specifier: MonitorSpecifier) -> Result<Monitor, String> {
    match specifier {
        MonitorSpecifier::ID(index) => {
            let a = monitors
                .iter::<Monitor>()
                .nth(index)
                .ok_or(format!("error matching monitor with id: {index}"))?
                .map_err(|e| format!("error matching monitor with id: {index}\nError: {e}"))?;
            Ok(a)
        }
        MonitorSpecifier::Name(name) => {
            for m in monitors.iter() {
                let m: Monitor =
                    m.map_err(|e| format!("error matching monitor with name: {name}\nError: {e}"))?;
                if m.connector()
                    .ok_or(format!("Fail to get monitor connector name: {m:?}"))?
                    == name
                {
                    return Ok(m);
                }
            }
            Err(format!("monitor with name: {name} not found"))
        }
    }
}

fn calculate_relative(cfg: &mut Config, max_size_raw: (i32, i32)) -> Result<(), String> {
    let max_size = match cfg.edge {
        Edge::Left | Edge::Right => (max_size_raw.0, max_size_raw.1),
        Edge::Top | Edge::Bottom => (max_size_raw.1, max_size_raw.0),
        _ => unreachable!(),
    };
    if let Ok(r) = cfg.width.get_rel() {
        cfg.width = NumOrRelative::Num(max_size.0 as f64 * r);
    };
    if let Ok(r) = cfg.height.get_rel() {
        cfg.height = NumOrRelative::Num(max_size.1 as f64 * r);
    };
    if let Ok(r) = cfg.extra_trigger_size.get_rel() {
        cfg.extra_trigger_size = NumOrRelative::Num((max_size.0 as f64 * r) as i32);
    };
    cfg.margins.iter_mut().for_each(|(e, n)| {
        if let Ok(r) = n.get_rel() {
            *n = match e {
                Edge::Left | Edge::Right => NumOrRelative::Num((r * max_size_raw.0 as f64) as i32),
                Edge::Top | Edge::Bottom => NumOrRelative::Num((r * max_size_raw.1 as f64) as i32),
                _ => unreachable!(),
            };
        };
    });

    // remember to check height since we didn't do it in `parse_config`
    // when passing only `rel_height`
    let w = cfg.width.get_num()?;
    let h = cfg.height.get_num()?;
    if w * 2. > h {
        Err(format!(
            "relative height detect: width * 2 must be <= height: {w} * 2 <= {h}",
        ))
    } else {
        Ok(())
    }
}

pub trait WindowInitializer {
    fn init_window(app: &Application, cfgs: GroupConfig);
}

struct ButtonItem {
    cfg: Config,
    monitor: Monitor,
}

fn create_buttons(app: &gtk::Application, button_items: Vec<ButtonItem>) {
    button_items.into_iter().for_each(|bti| {
        log::debug!("Final Config: {:?}", bti.cfg);
        let window = ui::new_window(app, bti.cfg);
        window.set_monitor(&bti.monitor);
        window.set_namespace("way-edges-widget");
    });
}
