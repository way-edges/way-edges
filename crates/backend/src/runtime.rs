use std::sync::atomic::{AtomicBool, AtomicPtr};

use tokio::runtime::Handle;

static TASK_HANDLER_INITED: AtomicBool = AtomicBool::new(false);
static TASK_HANDLER: AtomicPtr<Handle> = AtomicPtr::new(std::ptr::null_mut());
pub fn get_backend_runtime_handle() -> &'static Handle {
    unsafe {
        TASK_HANDLER
            .load(std::sync::atomic::Ordering::Relaxed)
            .as_ref()
            .unwrap()
    }
}

pub fn init_backend_runtime_handle() {
    let (created, is_created) = tokio::sync::oneshot::channel();

    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        TASK_HANDLER.store(
            Box::into_raw(Box::new(rt.handle().clone())),
            std::sync::atomic::Ordering::Relaxed,
        );
        TASK_HANDLER_INITED.store(true, std::sync::atomic::Ordering::Relaxed);

        created.send(()).unwrap();
        rt.block_on(std::future::pending::<()>());
    });

    is_created.blocking_recv().unwrap();
}
