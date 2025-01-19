use smithay_client_toolkit::shell::wlr_layer::Anchor;

use crate::{wayland::app::WidgetBuilder, window::WidgetContext};

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
    conf: &mut config::Config,
    builder: &mut WidgetBuilder<'a>,
) -> Box<dyn WidgetContext> {
    let monitor = builder.app.output_state.info(&builder.output).unwrap();
    let size = monitor.modes[0].dimensions;

    process_config(conf, size);

    match conf.widget.take().unwrap() {
        config::widgets::Widget::Btn(btn_config) => {
            log::debug!("initializing button");
            let w = button::init_widget(builder, size, &conf, btn_config);
            log::info!("initialized button");
            Box::new(w)
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
            Box::new(w)
        }
        config::widgets::Widget::WrapBox(box_config) => {
            log::debug!("initializing box");
            let w = wrapbox::init_widget(builder, &conf, box_config);
            log::info!("initialized box");
            Box::new(w)
        }
    }
}
