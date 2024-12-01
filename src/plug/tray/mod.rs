mod context;
mod event;

use std::{io::Cursor, path::PathBuf};

use cairo::ImageSurface;
use context::get_tray_context;
use gtk::{IconLookupFlags, IconPaintable, TextDirection};
use system_tray::item::{IconPixmap, StatusNotifierItem};

use crate::ui::draws::util::ImageData;

pub struct TrayItem {
    pub id: String,
    pub title: Option<String>,
    pub icon: TrayIcon,
    pub menu_path: Option<String>,
}
impl From<StatusNotifierItem> for TrayItem {
    fn from(value: StatusNotifierItem) -> Self {
        let id = value.id;
        let title = value.title;
        let icon_theme = get_tray_context().get_icon_theme();
        if let Some(theme) = value.icon_theme_path {
            if !icon_theme
                .search_path()
                .contains(&PathBuf::from(theme.clone()))
            {
                icon_theme.add_search_path(theme);
            }
        }
        let icon = if let Some(icon) = value.icon_name {
            TrayIcon::Name(icon)
        } else if let Some(icon) = value.icon_pixmap {
            TrayIcon::Pixmap(icon)
        } else {
            TrayIcon::default()
        };
        let menu_path = value.menu;
        Self {
            id,
            title,
            icon,
            menu_path,
        }
    }
}

pub enum TrayIcon {
    Name(String),
    Data(Vec<u8>),
    Pixmap(Vec<IconPixmap>),
}
impl Default for TrayIcon {
    fn default() -> Self {
        Self::Name("image-missing".to_string())
    }
}
impl TrayIcon {
    fn parse_icon_paintable(p: IconPaintable) -> ImageData {
        if let Some(path) = p.icon_name() {
            println!("icon name: {:?}", path);
        }
        todo!()
    }
    pub fn get_icon_with_size(self, size: i32, scale: i32, direction: TextDirection) -> ImageData {
        match self {
            TrayIcon::Name(name) => {
                // backup
                let icon_paintable = get_tray_context().get_icon_theme().lookup_icon(
                    &name,
                    &[],
                    size,
                    scale,
                    direction,
                    IconLookupFlags::empty(),
                );
                // Self::parse_icon_paintable(icon_paintable)

                {
                    ImageData {
                        width: 0,
                        height: 0,
                        stride: 0,
                        format: cairo::Format::Rgb24,
                        data: Vec::new(),
                    }
                }
            }
            TrayIcon::Data(vec) => ImageSurface::create_from_png(&mut Cursor::new(vec))
                .unwrap()
                .into(),
            TrayIcon::Pixmap(mut vec) => {
                if vec.is_empty() {
                    Self::default().get_icon_with_size(size, scale, direction)
                } else {
                    let a = vec.swap_remove(0);
                    ImageSurface::create_for_data(
                        a.pixels,
                        cairo::Format::ARgb32,
                        a.width,
                        a.height,
                        1,
                    )
                    .unwrap()
                    .into()
                }
            }
        }
    }
}

// represent the root menu(click on the icon and this menu gets triggered)
pub struct TrayMenu {
    id: u32,
    menus: Vec<Menu>,
}

impl From<system_tray::menu::TrayMenu> for TrayMenu {
    fn from(value: system_tray::menu::TrayMenu) -> Self {
        let id = value.id;
        let menus = value.submenus.vec_into();
        Self { id, menus }
    }
}

pub struct Menu {
    id: i32,
    label: Option<String>,
    enabled: bool,

    // icon can only be used once
    // we should preserve the data afterwards
    icon: Option<TrayIcon>,
    menu_type: MenuType,
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
