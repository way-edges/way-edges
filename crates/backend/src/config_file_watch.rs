use std::{sync::Arc, time::Duration};

use calloop::channel::Sender;
use config::get_config_path;
use futures_util::StreamExt;
use inotify::{Inotify, WatchMask};

use crate::runtime::get_backend_runtime_handle;

pub fn start_configuration_file_watcher(sender: Sender<()>) {
    get_backend_runtime_handle().spawn(async move {
        let inotify = Inotify::init().unwrap();
        let path = get_config_path().parent().unwrap();
        let config_name = get_config_path().file_name().unwrap();

        inotify
            .watches()
            .add(path, WatchMask::CREATE | WatchMask::MODIFY)
            .unwrap();

        let mut debouncer = None;

        let mut buffer = [0; 1024];
        let mut stream = inotify.into_event_stream(&mut buffer).unwrap();
        while let Some(event_or_error) = stream.next().await {
            let event = event_or_error.unwrap();
            let Some(name) = event.name else {
                continue;
            };
            if name.as_os_str() != config_name {
                continue;
            }

            let new_d = Arc::new(());
            let weak_d = Arc::downgrade(&new_d);
            debouncer.replace(new_d);

            let sender = sender.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(700)).await;
                if weak_d.upgrade().is_none() {
                    return;
                }
                sender.send(()).unwrap();
            });
        }
    });
}
