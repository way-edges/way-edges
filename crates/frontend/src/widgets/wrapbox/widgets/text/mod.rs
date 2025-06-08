mod draw;

use std::cell::UnsafeCell;
use std::{rc::Rc, time::Duration};

use calloop::channel::Sender;
use chrono::{Local, Utc};
use draw::TextDrawer;
use interval_task::runner::Runner;

use config::widgets::wrapbox::text::{TextConfig, TextPreset};
use util::shell::shell_cmd;

use super::super::box_traits::BoxedWidget;
use crate::widgets::wrapbox::BoxTemporaryCtx;

fn time_preset(
    s: Sender<String>,
    format: String,
    time_zone: Option<String>,
    update_interval: u64,
) -> Runner<()> {
    let f = move || {
        let time = time_zone
            .as_ref()
            .map_or(Local::now().naive_local(), |time_zone| {
                use chrono::TimeZone;
                let dt = Utc::now();
                let tz = time_zone.parse::<chrono_tz::Tz>().unwrap();
                tz.from_utc_datetime(&dt.naive_utc()).naive_local()
            });
        time.format(format.as_str()).to_string()
    };

    interval_task::runner::new_runner(
        Duration::from_millis(update_interval),
        || (),
        move |_| {
            s.send(f()).unwrap();
            false
        },
    )
}

fn custom_preset(s: Sender<String>, time: u64, cmd: String) -> Runner<()> {
    // ignore fail
    let f = move || shell_cmd(&cmd).unwrap_or_default();

    interval_task::runner::new_runner(
        Duration::from_millis(time),
        || (),
        move |_| {
            s.send(f()).unwrap();
            false
        },
    )
}

fn match_preset(preset: TextPreset, s: Sender<String>) -> Runner<()> {
    match preset {
        TextPreset::Time {
            format,
            time_zone,
            update_interval,
        } => time_preset(s, format, time_zone, update_interval),
        TextPreset::Custom {
            update_interval,
            cmd,
        } => custom_preset(s, update_interval, cmd),
    }
}

#[derive(Debug)]
pub struct TextCtx {
    #[allow(dead_code)]
    runner: Runner<()>,
    text: Rc<UnsafeCell<String>>,
    drawer: TextDrawer,
}

pub fn init_text(box_temp_ctx: &mut BoxTemporaryCtx, conf: TextConfig) -> impl BoxedWidget {
    let drawer = TextDrawer::new(&conf);

    let text = Rc::new(UnsafeCell::new(String::default()));
    let text_weak = Rc::downgrade(&text);
    let redraw_signal = box_temp_ctx.make_redraw_channel(move |_, msg| {
        let Some(text) = text_weak.upgrade() else {
            return;
        };
        unsafe { *text.get().as_mut().unwrap() = msg };
    });

    let mut runner = match_preset(conf.preset, redraw_signal);
    runner.start().unwrap();

    TextCtx {
        runner,
        text,
        drawer,
    }
}
