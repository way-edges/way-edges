use std::sync::{Arc, Mutex};

use system_tray::client::ActivateRequest;

use crate::runtime::get_backend_runtime_handle;

use super::{context::get_tray_context, item::Tray};

#[derive(Debug, Clone)]
pub enum TrayEventSignal {
    Add(Arc<String>, Arc<Mutex<Tray>>),
    Rm(Arc<String>),
    Update(Arc<String>),
}

pub fn tray_active_request(req: ActivateRequest) {
    get_backend_runtime_handle().spawn(async move {
        if let Err(e) = get_tray_context().lock().await.client.activate(req).await {
            let msg = format!("error requesting tray activation: {e:?}");
            log::error!("{msg}");
        }
    });
}

pub fn tray_about_to_show_menuitem(address: String, path: String, id: i32) {
    get_backend_runtime_handle().spawn(async move {
        if let Err(e) = get_tray_context()
            .lock()
            .await
            .client
            .about_to_show_menuitem(address, path, id)
            .await
        {
            let msg = format!("error requesting tray about to show: {e:?}");
            log::error!("{msg}");
        }
    });
}
