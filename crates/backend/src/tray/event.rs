use system_tray::client::ActivateRequest;
use util::notify_send;

use crate::runtime::get_backend_runtime_handle;

use super::context::get_tray_context;

pub type TrayEvent = (String, Event);

pub enum Event {
    TitleUpdate(Option<String>),
    IconUpdate(String),
    MenuNew(system_tray::menu::TrayMenu),
    ItemNew(Box<system_tray::item::StatusNotifierItem>),
    ItemRemove,
}

pub fn match_event(e: system_tray::client::Event) -> Option<TrayEvent> {
    match e {
        system_tray::client::Event::Add(id, status_notifier_item) => {
            Some((id, Event::ItemNew(status_notifier_item)))
        }
        system_tray::client::Event::Update(id, update_event) => match update_event {
            system_tray::client::UpdateEvent::Menu(tray_menu) => {
                Some((id, Event::MenuNew(tray_menu)))
            }
            system_tray::client::UpdateEvent::Title(title) => Some((id, Event::TitleUpdate(title))),
            // TODO: why icon update can only have name update
            system_tray::client::UpdateEvent::Icon(icon_path) => icon_path
                .filter(|name| !name.is_empty())
                .map(|name| (id, Event::IconUpdate(name))),

            // not implemented
            system_tray::client::UpdateEvent::AttentionIcon(_) => {
                log::warn!("NOT IMPLEMENTED ATTENTION ICON");
                None
            }
            system_tray::client::UpdateEvent::OverlayIcon(_) => {
                log::warn!("NOT IMPLEMENTED OVERLAY ICON");
                None
            }
            system_tray::client::UpdateEvent::Status(_) => {
                // no need
                log::warn!("NOT IMPLEMENTED STATUS");
                None
            }
            system_tray::client::UpdateEvent::Tooltip(_) => {
                // maybe some other time
                log::warn!("NOT IMPLEMENTED TOOLTIP");
                None
            }
            system_tray::client::UpdateEvent::MenuDiff(_) => {
                // ???
                log::warn!("NOT IMPLEMENTED MENU DIFF");
                None
            }
            system_tray::client::UpdateEvent::MenuConnect(_) => {
                // no need i think?
                log::warn!("NOT IMPLEMENTED MENU CONNECT");
                None
            }
        },
        system_tray::client::Event::Remove(id) => Some((id, Event::ItemRemove)),
    }
}

pub fn tray_active_request(req: ActivateRequest) {
    get_backend_runtime_handle().spawn(async move {
        if let Err(e) = get_tray_context().client.activate(req).await {
            let msg = format!("error requesting tray activation: {e}");
            log::error!("{msg}");
            notify_send("Tray activation", &msg, true);
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
            let msg = format!("error requesting tray activation: {e}");
            log::error!("{msg}");
            notify_send("Tray activation", &msg, true);
        }
    });
}
