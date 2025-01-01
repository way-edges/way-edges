use zbus::proxy;
use zbus::Connection;

use crate::backlight::match_device;
use crate::get_main_runtime_handle;

// NOTE: this dbus proxy takes 2 threads
// it'll create 1 thread which always on after first connection
#[proxy(interface = "org.freedesktop.login1.Session")]
trait BackLight {
    fn SetBrightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;
}

async fn get_proxy() -> zbus::Result<BackLightProxy<'static>> {
    let connection = Connection::system().await?;
    BackLightProxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1/session/auto",
    )
    .await
}

pub async fn set_brightness(device_name: &str, v: u32) -> zbus::Result<()> {
    let proxy = get_proxy().await?;
    proxy.SetBrightness("backlight", device_name, v).await?;
    Ok(())
}

pub fn set_backlight(device_name: Option<String>, p: f64) {
    let device = match_device(device_name).unwrap();
    let device_name = device.name().to_string();
    let v = (device.max() as f64) * p;
    get_main_runtime_handle().spawn(async move {
        if let Err(e) = set_brightness(&device_name, v as u32).await {
            log::error!("Error setting brightness: {e}");
        }
    });
}
