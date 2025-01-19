use std::sync::atomic::AtomicPtr;

use tokio::runtime::{Handle, LocalRuntime};

static LOCAL_RUNTIME: AtomicPtr<LocalRuntime> = AtomicPtr::new(std::ptr::null_mut());
static TASK_HANDLER: AtomicPtr<Handle> = AtomicPtr::new(std::ptr::null_mut());
pub fn get_backend_runtime() -> &'static LocalRuntime {
    unsafe {
        LOCAL_RUNTIME
            .load(std::sync::atomic::Ordering::Relaxed)
            .as_ref()
            .unwrap()
    }
}
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
        // let rt = tokio::runtime::Builder::new_current_thread()
        //     .enable_all()
        //     .build()
        //     .unwrap();
        let rt = tokio::runtime::LocalRuntime::new().unwrap();

        TASK_HANDLER.store(
            Box::into_raw(Box::new(rt.handle().clone())),
            std::sync::atomic::Ordering::Relaxed,
        );
        LOCAL_RUNTIME.store(
            Box::into_raw(Box::new(rt)),
            std::sync::atomic::Ordering::Relaxed,
        );

        created.send(()).unwrap();
        get_backend_runtime().block_on(std::future::pending::<()>());
    });

    is_created.blocking_recv().unwrap();
}
