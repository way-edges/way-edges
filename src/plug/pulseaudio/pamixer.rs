use std::{
    cell::Cell,
    hint::spin_loop,
    rc::Rc,
    sync::atomic::{AtomicPtr, Ordering},
    thread,
};

use async_channel::Sender;

use crate::plug::{common::shell_cmd, pulseaudio::pa::get_introspector};

use super::{OptionalSinkOrSource, OptionalSinkOrSourceDevice};

static PAMIXER_SIGNAL_SENDER: AtomicPtr<Sender<PaMixerSignal>> =
    AtomicPtr::new(std::ptr::null_mut());

pub fn send_pamixer_signal(
    msg: PaMixerSignal,
) -> Result<(), async_channel::SendError<PaMixerSignal>> {
    log::debug!("send signal");
    let r = unsafe {
        PAMIXER_SIGNAL_SENDER
            .load(Ordering::Acquire)
            .as_ref()
            .unwrap()
            .send_blocking(msg)
    };
    log::debug!("send signal done");
    r
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
        log::debug!("process pamixer signal");
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
                log::debug!("execute pamixer cmd");
                shell_cmd(cmd.join(" "));
                // gio::spawn_blocking(move || {
                // });
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
    let index = Rc::new(Cell::new(None));
    let is_done = Rc::new(Cell::new(false));
    use gtk::glib;
    let ss = s.to_string();
    get_introspector().get_sink_info_list(glib::clone!(
        #[strong]
        is_done,
        #[strong]
        index,
        move |l| {
            if is_done.get() {
                return;
            }
            match l {
                libpulse_binding::callbacks::ListResult::Item(si) => {
                    let a: &str = si.description.as_ref().unwrap();
                    if a.eq(&ss) {
                        index.set(Some(si.index));
                        is_done.set(true)
                    };
                }
                libpulse_binding::callbacks::ListResult::End => is_done.set(true),
                libpulse_binding::callbacks::ListResult::Error => {
                    log::error!("Get source info error");
                    is_done.set(true)
                }
            };
        }
    ));

    log::debug!("wait for match name index sink");
    while !is_done.get() {
        spin_loop();
    }
    log::debug!("wait for match name index sink done");
    if let Some(i) = index.get() {
        Ok(i)
    } else {
        Err(format!("no sink with name: {s}"))
    }
}
pub fn match_name_index_source(s: &str) -> Result<u32, String> {
    let index = Rc::new(Cell::new(None));
    let is_done = Rc::new(Cell::new(false));
    use gtk::glib;
    let ss = s.to_string();
    get_introspector().get_source_info_list(glib::clone!(
        #[strong]
        is_done,
        #[strong]
        index,
        move |l| {
            if is_done.get() {
                return;
            }
            match l {
                libpulse_binding::callbacks::ListResult::Item(si) => {
                    let a: &str = si.description.as_ref().unwrap();
                    if a.eq(&ss) {
                        index.set(Some(si.index));
                        is_done.set(true)
                    };
                }
                libpulse_binding::callbacks::ListResult::End => is_done.set(true),
                libpulse_binding::callbacks::ListResult::Error => is_done.set(true),
            };
        }
    ));
    while !is_done.get() {
        spin_loop();
    }
    if let Some(i) = index.get() {
        Ok(i)
    } else {
        Err(format!("no source with name: {s}"))
    }
}
