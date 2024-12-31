use async_channel::{Receiver, Sender};
use interval_task::runner::Runner;
use std::time::Duration;

use backend::system::{
    get_battery_info, get_cpu_info, get_disk_info, get_ram_info, get_swap_info, init_mem_info,
    init_system_info, register_disk_partition,
};
use config::widgets::wrapbox::ring::RingPreset;
use util::shell::shell_cmd;

fn from_kb(total: u64, avaibale: u64) -> (f64, f64, &'static str) {
    let mut c = 0;
    let mut total = total as f64;
    let mut avaibale = avaibale as f64;
    while total > 1000. && c < 3 {
        total /= 1000.;
        avaibale /= 1000.;
        c += 1;
    }
    let surfix = match c {
        0 => "KB",
        1 => "MB",
        2 => "GB",
        3 => "TB",
        _ => unreachable!(),
    };
    (total, avaibale, surfix)
}
fn from_kib(total: u64, avaibale: u64) -> (f64, f64, &'static str) {
    let mut c = 0;
    let mut total = total as f64;
    let mut avaibale = avaibale as f64;
    while total > 1024. && c < 3 {
        total /= 1024.;
        avaibale /= 1024.;
        c += 1;
    }
    let surfix = match c {
        0 => "KiB",
        1 => "MiB",
        2 => "GiB",
        3 => "TiB",
        _ => unreachable!(),
    };
    (total, avaibale, surfix)
}

macro_rules! new_runner {
    ($time:expr, $s:expr, $f:expr) => {
        interval_task::runner::new_runner(
            Duration::from_millis($time),
            || (),
            move |_| {
                $s.force_send($f()).unwrap();
                false
            },
        )
    };
}

fn ram(s: Sender<RunnerResult>) -> Runner<()> {
    init_mem_info();
    let f = || {
        let Some([ava, total]) = get_ram_info() else {
            return RunnerResult::default();
        };

        let (total, avaibale, surfix) = from_kib(total, ava);
        let progress = avaibale / total;
        let preset_text = format!(
            "{:.2}{surfix} / {:.2}{surfix} [{:.2}%]",
            avaibale,
            total,
            progress * 100.
        );

        RunnerResult {
            progress,
            preset_text,
        }
    };

    new_runner!(1000, s, f)
}

fn swap(s: Sender<RunnerResult>) -> Runner<()> {
    init_mem_info();
    let f = || {
        let Some([ava, total]) = get_swap_info() else {
            return RunnerResult::default();
        };

        let (total, avaibale, surfix) = from_kib(total, ava);
        let progress = avaibale / total;
        let preset_text = format!(
            "{:.2}{surfix} / {:.2}{surfix} [{:.2}%]",
            avaibale,
            total,
            progress * 100.
        );

        RunnerResult {
            progress,
            preset_text,
        }
    };

    new_runner!(1000, s, f)
}

fn cpu(s: Sender<RunnerResult>) -> Runner<()> {
    init_system_info();
    let f = || {
        let Some((progress, temp)) = get_cpu_info() else {
            return RunnerResult::default();
        };

        let text = format!("{:.2}% {temp:.2}°C", progress * 100.);
        RunnerResult {
            progress,
            preset_text: text,
        }
    };
    new_runner!(1000, s, f)
}

fn battery(s: Sender<RunnerResult>) -> Runner<()> {
    init_system_info();
    let f = || {
        let Some(progress) = get_battery_info() else {
            return RunnerResult::default();
        };

        let preset_text = format!("{:.2}%", progress * 100.);
        RunnerResult {
            progress,
            preset_text,
        }
    };
    new_runner!(1000, s, f)
}

fn disk(s: Sender<RunnerResult>, partition: String) -> Runner<()> {
    init_system_info();
    // TODO: unregister
    register_disk_partition(&partition);

    let f = move || {
        let Some((ava, total)) = get_disk_info(&partition) else {
            return RunnerResult::default();
        };

        let (total, avaibale, surfix) = from_kb(total, ava);
        let progress = avaibale / total;
        let preset_text = format!(
            "[Partition: {}] {:.2}{surfix} / {:.2}{surfix} [{:.2}%]",
            partition,
            avaibale,
            total,
            progress * 100.
        );
        RunnerResult {
            progress,
            preset_text,
        }
    };

    new_runner!(1000, s, f)
}

fn custom(s: Sender<RunnerResult>, interval_update: (u64, String)) -> Runner<()> {
    init_system_info();
    let (time, cmd) = interval_update;
    let f = move || {
        let Ok(progress) = shell_cmd(&cmd) else {
            return RunnerResult::default();
        };

        let progress = progress.trim().parse().unwrap_or(0.);

        // let text = template.parse(|parser| {
        //     if parser.name() == TEMPLATE_ARG_FLOAT {
        //         let parser = parser.downcast_mut::<TemplateArgFloatParser>().unwrap();
        //         parser.parse(progress)
        //     }
        // });

        RunnerResult {
            progress,
            preset_text: String::default(),
        }
    };
    new_runner!(time, s, f)
}

#[derive(Default, Debug)]
pub struct RunnerResult {
    pub progress: f64,
    pub preset_text: String,
}

pub fn parse_preset(preset: RingPreset) -> (Runner<()>, Receiver<RunnerResult>) {
    let (s, r) = async_channel::bounded(1);
    let runner = match preset {
        RingPreset::Ram => ram(s),
        RingPreset::Swap => swap(s),
        RingPreset::Cpu => cpu(s),
        RingPreset::Battery => battery(s),
        RingPreset::Disk { partition } => disk(s, partition),
        RingPreset::Custom { interval_update } => custom(s, interval_update),
    };

    (runner, r)
}
