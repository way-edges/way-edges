use system_tray::event::ActivateRequest;
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

pub fn match_event(e: system_tray::event::Event) -> Option<TrayEvent> {
    println!("EVENT: {e:#?}");
    match e {
        system_tray::event::Event::Add(id, status_notifier_item) => {
            Some((id, Event::ItemNew(status_notifier_item)))
        }
        system_tray::event::Event::Update(id, update_event) => match update_event {
            system_tray::event::UpdateEvent::Menu(tray_menu) => {
                Some((id, Event::MenuNew(tray_menu)))
            }
            system_tray::event::UpdateEvent::Title(title) => Some((id, Event::TitleUpdate(title))),
            // TODO: why icon update can only have name update
            system_tray::event::UpdateEvent::Icon(icon_path) => icon_path
                .filter(|name| !name.is_empty())
                .map(|name| (id, Event::IconUpdate(name))),

            // not implemented
            system_tray::event::UpdateEvent::AttentionIcon(_) => {
                log::warn!("NOT IMPLEMENTED ATTENTION ICON");
                None
            }
            system_tray::event::UpdateEvent::OverlayIcon(_) => {
                log::warn!("NOT IMPLEMENTED OVERLAY ICON");
                None
            }
            system_tray::event::UpdateEvent::Status(_) => {
                // no need
                log::warn!("NOT IMPLEMENTED STATUS");
                None
            }
            system_tray::event::UpdateEvent::Tooltip(_) => {
                // maybe some other time
                log::warn!("NOT IMPLEMENTED TOOLTIP");
                None
            }
            system_tray::event::UpdateEvent::MenuDiff(_) => {
                // ???
                log::warn!("NOT IMPLEMENTED MENU DIFF");
                None
            }
            system_tray::event::UpdateEvent::MenuConnect(_) => {
                // no need i think?
                log::warn!("NOT IMPLEMENTED MENU CONNECT");
                None
            }
        },
        system_tray::event::Event::Remove(id) => Some((id, Event::ItemRemove)),
    }
}

pub fn tray_request_event(req: ActivateRequest) {
    get_backend_runtime_handle().spawn(async move {
        if let Err(e) = get_tray_context().client.activate(req).await {
            let msg = format!("error requesting tray activation: {e}");
            log::error!("{msg}");
            notify_send("Tray activation", &msg, true);
        }
    });
}
