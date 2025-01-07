use std::{sync::OnceLock, time::Duration};

use async_channel::{Receiver, Sender};
use config::get_config_path;
use notify::{EventKind, INotifyWatcher};
use util::notify_send;
// use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, NoCache};

fn file_monitor_error(msg: String) {
    notify_send("Way-edges file monitor", &msg, true);
    log::error!("{msg}");
}

pub fn init_config_file_monitor() -> Receiver<()> {
    static FILE_MONITOR: OnceLock<Debouncer<INotifyWatcher, NoCache>> = OnceLock::new();
    let (s, r) = async_channel::bounded(1);
    FILE_MONITOR.set(file_monitor(s)).unwrap();
    r
}

fn file_monitor(s: Sender<()>) -> Debouncer<INotifyWatcher, NoCache> {
    let res = new_debouncer(
        Duration::from_millis(700),
        None,
        // move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>>| match res {
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                log::debug!("{events:?}");
                let config_changed = events.into_iter().any(|de| {
                    match de.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => de
                            .paths
                            .iter()
                            .any(|path_buf| path_buf.as_path().eq(get_config_path())),
                        _ => false, // EventKind::Any => todo!(),
                                    // EventKind::Access(access_kind) => todo!(),
                                    // EventKind::Other => todo!(),
                    }
                });
                if config_changed {
                    s.try_send(()).unwrap()
                };
            }
            Err(e) => file_monitor_error(format!("watch error: {:?}", e)),
        },
    )
    .and_then(|mut debouncer| {
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        debouncer.watch(
            get_config_path().parent().unwrap(),
            notify::RecursiveMode::NonRecursive,
        )?;
        Ok(debouncer)
    });

    res.unwrap()
}
