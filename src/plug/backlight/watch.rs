use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicPtr, Ordering},
};

use async_channel::{Receiver, Sender};
use notify::{INotifyWatcher, Watcher};

type WatcherCtx = (INotifyWatcher, HashMap<String, Sender<Signal>>);
pub type Signal = Result<(), String>;

// static WATCHER: OnceLock<INotifyWatcher> = OnceLock::new();
static WATCHER: AtomicPtr<WatcherCtx> = AtomicPtr::new(std::ptr::null_mut());

fn get_watcher_ptr() -> *mut WatcherCtx {
    WATCHER.load(Ordering::Acquire)
}

fn notify_path(p: &str) {
    unsafe {
        if let Some(s) = get_watcher_ptr().as_mut().unwrap().1.get(p) {
            if let Err(e) = s.force_send(Ok(())) {
                log::error!("Error sending signal to watcher: {e}");
            };
        }
    }
}
fn notify_error(e: String) {
    unsafe {
        get_watcher_ptr()
            .as_mut()
            .unwrap()
            .1
            .iter()
            .for_each(|(_, s)| {
                s.force_send(Err(e.clone())).ok();
            });
    }
}

fn get_watcher() -> Result<&'static mut (INotifyWatcher, HashMap<String, Sender<Signal>>), String> {
    if get_watcher_ptr().is_null() {
        let watcher = notify::recommended_watcher(move |e: notify::Result<notify::Event>| {
            match e {
                Ok(event) => {
                    if event.kind.is_modify() {
                        let p = event.paths.iter().find(|p| p.ends_with("brightness"));
                        if let Some(p) = p {
                            notify_path(p.to_str().unwrap());
                        }
                    }
                }
                Err(e) => {
                    notify_error(e.to_string());
                }
            };
        })
        .map_err(|e| format!("Failed to create watcher for backlight: {e}"))?;
        WATCHER.store(
            Box::into_raw(Box::new((watcher, HashMap::new()))),
            Ordering::Release,
        );
    }
    unsafe { Ok(get_watcher_ptr().as_mut().unwrap()) }
}

#[derive(Debug)]
pub struct WatchCtx {
    pub path_buf: PathBuf,
    pub r: Receiver<Signal>,
}
impl WatchCtx {
    fn new(p: &Path, r: Receiver<Signal>) -> Self {
        Self {
            path_buf: p.to_path_buf(),
            r,
        }
    }
}
impl Drop for WatchCtx {
    fn drop(&mut self) {
        unwatch(&self.path_buf).unwrap();
    }
}

pub fn watch(p: &Path) -> Result<WatchCtx, String> {
    log::info!("backlight watching path {:?}", p);
    let (w, h) = get_watcher()?;
    w.watch(p, notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch path {p:?}: {e}"))?;
    let (s, r) = async_channel::bounded(1);
    h.insert(p.to_string_lossy().to_string(), s);
    Ok(WatchCtx::new(p, r))
}

pub fn unwatch(p: &Path) -> Result<(), String> {
    log::info!("backlight unwatching path {:?}", p);
    let (w, h) = get_watcher()?;
    w.unwatch(p)
        .map_err(|e| format!("Failed to unwatch path {p:?}: {e}"))?;
    h.remove(&p.to_string_lossy().to_string());
    Ok(())
}
