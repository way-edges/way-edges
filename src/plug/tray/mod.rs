mod context;
mod event;
pub mod icon;

pub use context::{register_tray, tray_update_item_theme_search_path, unregister_tray};
pub use event::Event;

// represent the root menu(click on the icon and this menu gets triggered)
pub struct TrayMenu {
    pub id: u32,
    pub menus: Vec<Menu>,
}

impl From<system_tray::menu::TrayMenu> for TrayMenu {
    fn from(value: system_tray::menu::TrayMenu) -> Self {
        let id = value.id;
        let menus = value.submenus.vec_into();
        Self { id, menus }
    }
}

pub struct Menu {
    pub id: i32,
    pub label: Option<String>,
    pub enabled: bool,

    // icon can only be used once
    // we should preserve the data afterwards
    pub icon: Option<TrayIcon>,
    pub menu_type: MenuType,
}

impl From<system_tray::menu::MenuItem> for Menu {
    fn from(value: system_tray::menu::MenuItem) -> Self {
        let id = value.id;
        let label = value.label;
        let enabled = value.enabled;

        #[allow(clippy::manual_map)]
        let icon = if let Some(icon) = value.icon_name {
            Some(TrayIcon::Name(icon))
        } else if let Some(icon) = value.icon_data {
            Some(TrayIcon::Data(icon))
        } else {
            None
        };

        let menu_type = match value.menu_type {
            system_tray::menu::MenuType::Separator => MenuType::Separator,
            system_tray::menu::MenuType::Standard => {
                match value.toggle_type {
                    system_tray::menu::ToggleType::Checkmark => {
                        MenuType::Check(match value.toggle_state {
                            system_tray::menu::ToggleState::On => true,
                            system_tray::menu::ToggleState::Off => false,
                            system_tray::menu::ToggleState::Indeterminate => {
                                log::error!("THIS SHOULD NOT HAPPEN. menu item has toggle but not toggle state");
                                // ???
                                false
                            }
                        })
                    }
                    system_tray::menu::ToggleType::Radio => {
                        MenuType::Radio(match value.toggle_state {
                            system_tray::menu::ToggleState::On => true,
                            system_tray::menu::ToggleState::Off => false,
                            system_tray::menu::ToggleState::Indeterminate => {
                                log::error!("THIS SHOULD NOT HAPPEN. menu item has toggle but not toggle state");
                                // ???
                                false
                            }
                        })
                    }
                    system_tray::menu::ToggleType::CannotBeToggled => {
                        if !value.submenu.is_empty() {
                            MenuType::Parent(value.submenu.vec_into())
                        } else {
                            MenuType::Normal
                        }
                    }
                }
            }
        };

        Self {
            id,
            label,
            enabled,
            icon,
            menu_type,
        }
    }
}

pub enum MenuType {
    Radio(bool),
    Check(bool),
    Parent(Vec<Menu>),
    Separator,
    Normal,
}

pub trait VecInto<D> {
    fn vec_into(self) -> Vec<D>;
}

impl<E, D> VecInto<D> for Vec<E>
where
    D: From<E>,
{
    fn vec_into(self) -> Vec<D> {
        self.into_iter().map(std::convert::Into::into).collect()
    }
}
