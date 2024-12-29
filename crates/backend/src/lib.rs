use std::sync::atomic::AtomicPtr;

pub mod backlight;
pub mod config_file_watch;
pub mod hypr_workspace;
pub mod monitor;
pub mod pulseaudio;
pub mod system;
pub mod tray;

static MAIN_RUNTIME_HANDLE: AtomicPtr<tokio::runtime::Handle> =
    AtomicPtr::new(std::ptr::null_mut());

pub fn get_main_runtime_handle() -> &'static tokio::runtime::Handle {
    unsafe {
        MAIN_RUNTIME_HANDLE
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .unwrap()
    }
}

pub fn set_main_runtime_handle() {
    let main_runtime_handle = tokio::runtime::Handle::current();
    MAIN_RUNTIME_HANDLE.store(
        Box::into_raw(Box::new(main_runtime_handle)),
        std::sync::atomic::Ordering::Release,
    );
}
