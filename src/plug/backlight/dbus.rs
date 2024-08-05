use zbus::Connection;
use zbus::{proxy, Result};

// NOTE: this dbus proxy takes 2 threads
// it'll create 1 thread which always on after first connection
#[proxy(interface = "org.freedesktop.login1.Session")]
trait BackLight {
    fn SetBrightness(&self, subsystem: &str, name: &str, brightness: u32) -> Result<()>;
}

async fn get_proxy() -> Result<BackLightProxy<'static>> {
    let connection = Connection::system().await?;
    BackLightProxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1/session/auto",
    )
    .await
}

pub async fn set_brightness(device_name: &str, p: u32) -> Result<()> {
    let proxy = get_proxy().await?;
    proxy.SetBrightness("backlight", device_name, p).await?;
    Ok(())
}
