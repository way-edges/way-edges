use super::TrayIcon;

pub type TrayEvent = (String, Event);

pub enum Event {
    TitleUpdate(String),
    IconUpdate(TrayIcon),
    MenuNew(super::TrayMenu),
    ItemNew(super::TrayItem),
    ItemRemove,
}

pub fn match_event(e: system_tray::client::Event) -> Option<TrayEvent> {
    match e {
        system_tray::client::Event::Add(id, status_notifier_item) => {
            Some((id, Event::ItemNew((*status_notifier_item).into())))
        }
        system_tray::client::Event::Update(id, update_event) => match update_event {
            system_tray::client::UpdateEvent::Menu(tray_menu) => {
                Some((id, Event::MenuNew(tray_menu.into())))
            }
            system_tray::client::UpdateEvent::Title(title) => {
                Some((id, Event::TitleUpdate(title.unwrap_or_default())))
            }
            system_tray::client::UpdateEvent::Icon(icon_path) => {
                let icon = icon_path.map_or(TrayIcon::default(), TrayIcon::Name);
                Some((id, Event::IconUpdate(icon)))
            }

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