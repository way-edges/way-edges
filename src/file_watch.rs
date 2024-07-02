use std::{process, sync::OnceLock, time::Duration};

use crate::config::get_config_path;
use crate::notify_send;
use async_channel::{Receiver, Sender};
use notify::INotifyWatcher;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};

fn file_monitor_error(msg: String) {
    notify_send("Way-edges file monitor", &msg, true);
    log::error!("{msg}");
    process::exit(1)
}

static mut FILE_MONITOR: OnceLock<Debouncer<INotifyWatcher>> = OnceLock::new();
pub fn init_file_monitor() -> Receiver<()> {
    unsafe {
        let (s, r) = async_channel::bounded(1);
        FILE_MONITOR.set(file_monitor(s)).unwrap();
        r
    }
}
pub fn start_watch_file() {
    unsafe {
        let watcher = FILE_MONITOR.get_mut().unwrap().watcher();
        watcher
            .watch(
                get_config_path().parent().unwrap(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();
    }
}
pub fn stop_watch_file() {
    unsafe {
        let watcher = FILE_MONITOR.get_mut().unwrap().watcher();
        watcher
            .unwatch(get_config_path().parent().unwrap())
            .unwrap();
    }
}

pub fn file_monitor(s: Sender<()>) -> Debouncer<INotifyWatcher> {
    use notify::Result;
    let res = new_debouncer(
        Duration::from_millis(700),
        // move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>>| match res {
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>>| match res {
            Ok(event) => {
                log::debug!("{event:?}");
                let config_changed = event.into_iter().any(|de| {
                    if de.kind == DebouncedEventKind::Any {
                        de.path.as_path().eq(get_config_path())
                    } else {
                        false
                    }
                });
                if config_changed {
                    if let Err(e) = s.try_send(()) {
                        if let async_channel::TrySendError::Closed(_) = e {
                            file_monitor_error(format!(
                                "Failed to send file watcher event: Error: {e}"
                            ));
                        }
                    }
                };
            }
            Err(e) => file_monitor_error(format!("watch error: {:?}", e)),
        },
    )
    .and_then(|mut debouncer| {
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        debouncer.watcher().watch(
            get_config_path().parent().unwrap(),
            notify::RecursiveMode::NonRecursive,
        )?;
        Ok(debouncer)
    });
    match res {
        Ok(w) => w,
        Err(e) => panic!("Failed to create file watcher: Error: {e}"),
    }
}
