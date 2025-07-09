use std::sync::Arc;

use system_tray::client::ActivateRequest;

use crate::runtime::get_backend_runtime_handle;

use super::{
    context::{get_tray_context, TrayMap},
    item::{Icon, RootMenu, Tray},
};

#[derive(Debug, Clone)]
pub enum TrayEventSignal {
    Add(Arc<String>),
    Rm(Arc<String>),
    Update(Arc<String>),
}

impl TrayMap {
    pub(super) fn handle_event(
        &mut self,
        e: system_tray::client::Event,
    ) -> Option<TrayEventSignal> {
        match e {
            system_tray::client::Event::Add(dest, status_notifier_item) => {
                let item = Tray::new(*status_notifier_item);
                let dest = Arc::new(dest);
                self.inner.insert(dest.clone(), item);
                Some(TrayEventSignal::Add(dest))
            }
            system_tray::client::Event::Remove(id) => {
                self.inner.remove(&id);
                Some(TrayEventSignal::Rm(Arc::new(id)))
            }
            system_tray::client::Event::Update(id, update_event) => {
                let need_update = match update_event {
                    system_tray::client::UpdateEvent::Menu(tray_menu) => {
                        if let Some(tray) = self.inner.get_mut(&id) {
                            tray.update_menu(RootMenu::from_tray_menu(tray_menu))
                        }
                        true
                    }
                    system_tray::client::UpdateEvent::Title(title) => self
                        .inner
                        .get_mut(&id)
                        .map(|tray| tray.update_title(title))
                        .unwrap_or_default(),
                    system_tray::client::UpdateEvent::Icon {
                        icon_name,
                        icon_pixmap,
                    } => {
                        let icon = icon_name
                            .filter(|name| !name.is_empty())
                            .map(Icon::Named)
                            .or_else(|| {
                                icon_pixmap
                                    .filter(|pixmap| !pixmap.is_empty())
                                    .map(Icon::Pixmap)
                            });

                        self.inner
                            .get_mut(&id)
                            .map(|tray| tray.update_icon(icon))
                            .unwrap_or_default()
                    }

                    // not implemented
                    system_tray::client::UpdateEvent::AttentionIcon(_) => {
                        log::warn!("NOT IMPLEMENTED ATTENTION ICON");
                        false
                    }
                    system_tray::client::UpdateEvent::OverlayIcon(_) => {
                        log::warn!("NOT IMPLEMENTED OVERLAY ICON");
                        false
                    }
                    system_tray::client::UpdateEvent::Status(_) => {
                        // no need
                        log::warn!("NOT IMPLEMENTED STATUS");
                        false
                    }
                    system_tray::client::UpdateEvent::Tooltip(_) => {
                        // maybe some other time
                        log::warn!("NOT IMPLEMENTED TOOLTIP");
                        false
                    }
                    system_tray::client::UpdateEvent::MenuDiff(diffs) => {
                        if let Some(tray) = self.inner.get_mut(&id) {
                            diffs
                                .into_iter()
                                .for_each(|diff| tray.update_menu_item(diff));
                        }
                        true
                    }
                    system_tray::client::UpdateEvent::MenuConnect(_) => {
                        // no need i think?
                        log::warn!("NOT IMPLEMENTED MENU CONNECT");
                        false
                    }
                };

                if need_update {
                    Some(TrayEventSignal::Update(Arc::new(id)))
                } else {
                    None
                }
            }
        }
    }
}

pub fn tray_active_request(req: ActivateRequest) {
    get_backend_runtime_handle().spawn(async move {
        if let Err(e) = get_tray_context().client.activate(req).await {
            let msg = format!("error requesting tray activation: {e:?}");
            log::error!("{msg}");
        }
    });
}

pub fn tray_about_to_show_menuitem(address: String, path: String, id: i32) {
    get_backend_runtime_handle().spawn(async move {
        if let Err(e) = get_tray_context()
            .client
            .about_to_show_menuitem(address, path, id)
            .await
        {
            let msg = format!("error requesting tray about to show: {e:?}");
            log::error!("{msg}");
        }
    });
}
