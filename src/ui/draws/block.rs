use gtk::{cairo::LinearGradient, gdk::RGBA};

pub fn gen_linear_grandient(
    start_point: (f64, f64),
    end_point: (f64, f64),
    start_color: RGBA,
    end_color: RGBA,
) -> LinearGradient {
    let rg = LinearGradient::new(start_point.0, start_point.1, end_point.0, end_point.1);
    rg.add_color_stop_rgba(
        0.,
        start_color.red().into(),
        start_color.green().into(),
        start_color.blue().into(),
        start_color.alpha().into(),
    );
    rg.add_color_stop_rgba(
        1.,
        end_color.red().into(),
        end_color.green().into(),
        end_color.blue().into(),
        end_color.alpha().into(),
    );
    rg
}
