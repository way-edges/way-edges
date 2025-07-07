use std::{sync::Arc, time::Duration};

use calloop::channel::Sender;
use config::get_config_path;
use futures_util::StreamExt;
use inotify::{EventMask, Inotify, WatchMask};
use log::info;

use crate::runtime::get_backend_runtime_handle;

pub fn start_configuration_file_watcher(sender: Sender<()>) {
    get_backend_runtime_handle().spawn(async move {
        let mut debouncer = None;

        loop {
            let inotify = Inotify::init().unwrap();
            let file_path = get_config_path();

            inotify
                .watches()
                .add(
                    file_path,
                    WatchMask::CREATE
                        | WatchMask::MODIFY
                        | WatchMask::DELETE
                        | WatchMask::DELETE_SELF
                        | WatchMask::MOVE
                        | WatchMask::MOVE_SELF
                        | WatchMask::ATTRIB,
                )
                .unwrap();

            let mut buffer = [0; 1024];
            let mut stream = inotify.into_event_stream(&mut buffer).unwrap();
            while let Some(event_or_error) = stream.next().await {
                let event = event_or_error.unwrap();
                info!("Received inotify event: {event:?}");

                let new_d = Arc::new(());
                let weak_d = Arc::downgrade(&new_d);
                debouncer.replace(new_d);

                let sender = sender.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(700)).await;
                    if weak_d.upgrade().is_none() {
                        return;
                    }
                    println!("aaa");
                    sender.send(()).unwrap();
                });

                if event.mask == EventMask::IGNORED {
                    break;
                }
            }
        }
    });
}
