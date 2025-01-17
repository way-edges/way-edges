use backend::monitor::get_monitor_context;
use gtk::{gdk::Monitor, prelude::MonitorExt};
use smithay_client_toolkit::shell::wlr_layer::Anchor;

use crate::{
    wayland::app::WidgetBuilder,
    window::{WidgetContext, WindowContext, WindowContextBuilder},
};

mod button;
mod hypr_workspace;
mod slide;
mod wrapbox;

fn process_config(conf: &mut config::Config, size: (i32, i32)) {
    // margins
    macro_rules! calculate_margins {
        ($m:expr, $s:expr) => {
            if $m.is_relative() {
                $m.calculate_relative($s as f64);
            }
        };
    }
    calculate_margins!(conf.margins.left, size.0);
    calculate_margins!(conf.margins.right, size.0);
    calculate_margins!(conf.margins.top, size.1);
    calculate_margins!(conf.margins.bottom, size.1);

    // extra
    if conf.extra_trigger_size.is_relative() {
        let max = match conf.edge {
            Anchor::LEFT | Anchor::RIGHT => size.0,
            Anchor::TOP | Anchor::BOTTOM => size.1,
            _ => unreachable!(),
        };
        conf.extra_trigger_size.calculate_relative(max as f64);
    }
}

pub fn init_widget<'a>(
    mut conf: config::Config,
    builder: &mut WidgetBuilder<'a>,
) -> Result<WindowContext, String> {
    let monitor = builder.app.output_state.info(&builder.output).unwrap();
    let size = monitor.modes[0].dimensions;

    process_config(&mut conf, size);

    let widget_ctx = match conf.widget.take().unwrap() {
        config::widgets::Widget::Btn(btn_config) => {
            log::debug!("initializing button");
            let w = button::init_widget(builder, size, &conf, btn_config);
            log::info!("initialized button");
            w.make_rc()
        }
        config::widgets::Widget::Slider(slide_config) => {
            log::debug!("initializing slider");
            let w = slide::init_widget(builder, size, &conf, slide_config);
            log::info!("initialized slider");
            w
        }
        config::widgets::Widget::HyprWorkspace(hypr_workspace_config) => {
            log::debug!("initializing hypr-workspace");
            let w = hypr_workspace::init_widget(builder, size, &conf, hypr_workspace_config);
            log::info!("initialized hypr-workspace");
            w.make_rc()
        }
        config::widgets::Widget::WrapBox(box_config) => {
            log::debug!("initializing box");
            let w = wrapbox::init_widget(builder, size, &conf, box_config);
            log::info!("initialized box");
            w.make_rc()
        }
    };

    let window = builder.build(conf, widget_ctx);

    window.show();

    Ok(window)
}
