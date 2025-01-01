use backend::monitor::get_monitor_context;
use gtk::{gdk::Monitor, prelude::MonitorExt};

use crate::window::WindowContext;

mod button;
mod hypr_workspace;
mod slide;
mod wrapbox;

fn process_config(conf: &mut config::Config, monitor: &Monitor) {
    let geom = monitor.geometry();
    let size = (geom.width(), geom.height());

    // margins
    conf.margins.iter_mut().for_each(|(edge, n)| {
        if !n.is_relative() {
            return;
        }

        let max = match edge {
            gtk4_layer_shell::Edge::Left | gtk4_layer_shell::Edge::Right => size.0,
            gtk4_layer_shell::Edge::Top | gtk4_layer_shell::Edge::Bottom => size.1,
            _ => unreachable!(),
        };
        n.calculate_relative(max as f64);
    });

    // extra
    if conf.extra_trigger_size.is_relative() {
        let max = match conf.edge {
            gtk4_layer_shell::Edge::Left | gtk4_layer_shell::Edge::Right => size.0,
            gtk4_layer_shell::Edge::Top | gtk4_layer_shell::Edge::Bottom => size.1,
            _ => unreachable!(),
        };
        conf.extra_trigger_size.calculate_relative(max as f64);
    }

    // frame_rate
    if conf.frame_rate.is_none() {
        conf.frame_rate = Some(monitor.refresh_rate());
    }
}

pub fn init_widget(
    app: &gtk::Application,
    mut conf: config::Config,
) -> Result<WindowContext, String> {
    let monitor = get_monitor_context()
        .get_monitor(&conf.monitor)
        .ok_or(format!("Failed to get monitor {:?}", conf.monitor))?;

    process_config(&mut conf, monitor);

    let mut window = WindowContext::new(app, monitor, &conf)?;

    match conf.widget.take().unwrap() {
        config::widgets::Widget::Btn(btn_config) => {
            log::debug!("initializing button");
            button::init_widget(&mut window, monitor, conf, btn_config);
            log::info!("initialized button");
        }
        config::widgets::Widget::Slider(slide_config) => {
            log::debug!("initializing slider");
            slide::init_widget(&mut window, monitor, conf, slide_config);
            log::info!("initialized slider");
        }
        config::widgets::Widget::HyprWorkspace(hypr_workspace_config) => {
            log::debug!("initializing hypr-workspace");
            hypr_workspace::init_widget(&mut window, monitor, conf, hypr_workspace_config);
            log::info!("initialized hypr-workspace");
        }
        config::widgets::Widget::WrapBox(box_config) => {
            log::debug!("initializing box");
            wrapbox::init_widget(&mut window, monitor, conf, box_config);
            log::info!("initialized box");
        }
    };

    window.show();

    Ok(window)
}
