use std::sync::atomic::{AtomicPtr, Ordering};

use zbus::Connection;
use zbus::{proxy, Result};

// struct BackLight;

#[proxy(interface = "org.freedesktop.login1.Session")]
trait BackLight {
    fn SetBrightness(&self, subsystem: &str, name: &str, brightness: u32) -> Result<()>;
}

static PROXY: AtomicPtr<BackLightProxy> = AtomicPtr::new(std::ptr::null_mut());

fn get_proxy_pointer() -> *mut BackLightProxy<'static> {
    PROXY.load(std::sync::atomic::Ordering::Acquire)
}

async fn get_proxy() -> Result<&'static mut BackLightProxy<'static>> {
    let p = get_proxy_pointer();
    if p.is_null() {
        let connection = Connection::system().await?;

        PROXY.store(
            Box::into_raw(Box::new(
                BackLightProxy::new(
                    &connection,
                    "org.freedesktop.login1",
                    "/org/freedesktop/login1/session/auto",
                )
                .await?,
            )),
            Ordering::Release,
        );
    };
    unsafe { Ok(p.as_mut().unwrap()) }
}

pub async fn set_brightness(device_name: &str, p: u32) -> Result<()> {
    let proxy = get_proxy().await?;
    proxy.SetBrightness("backlight", device_name, p).await?;
    Ok(())
}
