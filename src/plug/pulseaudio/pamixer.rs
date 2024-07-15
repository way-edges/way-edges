use std::{
    sync::atomic::{AtomicPtr, Ordering},
    thread,
    time::Instant,
};

use async_channel::Sender;

use crate::plug::common::shell_cmd;

use super::{OptionalSinkOrSource, OptionalSinkOrSourceDevice};

static PAMIXER_SIGNAL_SENDER: AtomicPtr<Sender<PaMixerSignal>> =
    AtomicPtr::new(std::ptr::null_mut());

pub fn send_pamixer_signal(
    msg: PaMixerSignal,
) -> Result<(), async_channel::SendError<PaMixerSignal>> {
    unsafe {
        PAMIXER_SIGNAL_SENDER
            .load(Ordering::Acquire)
            .as_ref()
            .unwrap()
            .send_blocking(msg)
    }
}

// pamixser thread
type PaMixerSignal = (OptionalSinkOrSource, PaMixerSignalInfo);
pub enum PaMixerSignalInfo {
    Volume(f64),
    Mute(bool),
}
impl PaMixerSignalInfo {
    pub fn vol(v: f64) -> Self {
        Self::Volume(v)
    }
    pub fn mute(m: bool) -> Self {
        Self::Mute(m)
    }
}

pub fn init_pamixser_thread() {
    let (s, r) = async_channel::bounded::<PaMixerSignal>(1);
    thread::spawn(move || loop {
        let res = r.recv_blocking();
        match res {
            Ok((sink_or_source, info)) => {
                let mut cmd = vec!["pamixer".to_string()];
                match sink_or_source.0.as_ref() {
                    OptionalSinkOrSourceDevice::Sink(os) => {
                        if let Some(s) = os {
                            match match_name_index_sink(s) {
                                Ok(index) => {
                                    cmd.push(format!("--sink {index}"));
                                }
                                Err(e) => {
                                    log::error!("Error getting sink index: {e}");
                                }
                            };
                        }
                    }
                    OptionalSinkOrSourceDevice::Source(os) => {
                        if let Some(s) = os {
                            match match_name_index_source(s) {
                                Ok(index) => {
                                    cmd.push(format!("--source {index}"));
                                }
                                Err(e) => {
                                    log::error!("Error getting sink index: {e}");
                                }
                            };
                        } else {
                            cmd.push("--default-source".to_string());
                        }
                    }
                };
                match info {
                    PaMixerSignalInfo::Volume(f) => {
                        cmd.push(format!("--set-volume {}", (f * 100.) as u32));
                    }
                    PaMixerSignalInfo::Mute(m) => cmd.push(
                        match m {
                            true => "--mute",
                            false => "--unmute",
                        }
                        .to_string(),
                    ),
                };
                gio::spawn_blocking(move || {
                    shell_cmd(cmd.join(" "));
                });
            }
            Err(e) => {
                log::error!("Error receiving pamixser signal, quiting: {e}");
                break;
            }
        }
    });

    PAMIXER_SIGNAL_SENDER.store(Box::into_raw(Box::new(s)), Ordering::Release);
}

pub fn match_name_index_sink(s: &str) -> Result<u32, String> {
    let cmd = format!("pamixer --list-sinks | grep \"\\\"{s}\\\"\" | awk '{{print $1}}'");
    let o = shell_cmd(cmd.to_string())?;
    log::debug!("match sink name result: {o}");
    // let i = match_index(o.lines(), &s).ok_or(format!("no sink with name: {s}"))?;
    let i = to_index(&o).map_err(|_| "fail to parse sink index: {o}")?;
    Ok(i)
}
pub fn match_name_index_source(s: &str) -> Result<u32, String> {
    let cmd = format!("pamixer --list-sources | grep \"\\\"{s}\\\"\" | awk '{{print $1}}'");
    let o = shell_cmd(cmd.to_string())?;
    log::debug!("match source name result: {o}");
    // let i = match_index(o.lines(), &s).ok_or(format!("no source with name: {s}"))?;
    let i = to_index(&o).map_err(|_| "fail to parse source index: {o}")?;
    Ok(i)
}

fn to_index(inp: &str) -> Result<u32, std::num::ParseIntError> {
    let a = inp
        .strip_suffix("\r\n")
        .or(inp.strip_suffix('\n'))
        .unwrap_or(inp);
    use std::str::FromStr;
    u32::from_str(a)
}

// fn match_index(lines: Lines, s: &str) -> Option<u32> {
//     let mut index = 0;
//     for line in lines {
//         if line.contains(s) {
//             return Some(index);
//         }
//         index += 1;
//     }
//     None
// }
