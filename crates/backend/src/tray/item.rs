use cairo::ImageSurface;
use system_tray::item::{IconPixmap, StatusNotifierItem};

use super::icon::{
    fallback_icon, parse_icon_given_data, parse_icon_given_name, parse_icon_given_pixmaps,
    IconThemeNameOrPath,
};

#[derive(Debug)]
pub enum Icon {
    Named {
        name: String,
        theme_path: Option<String>,
    },
    PngData(Vec<u8>),
    Pixmap(Vec<IconPixmap>),
    Empty,
}
impl Icon {
    pub fn draw_icon(&self, size: i32, theme: Option<&str>) -> Option<ImageSurface> {
        match self {
            Icon::Named { name, theme_path } => {
                let theme_or_path = theme_path
                    .as_ref()
                    .map(|f| IconThemeNameOrPath::Path(f))
                    .unwrap_or_else(|| IconThemeNameOrPath::Name(theme));
                parse_icon_given_name(name, size, theme_or_path)
                    .or_else(|| fallback_icon(size, theme))
            }
            Icon::PngData(items) => parse_icon_given_data(items, size),
            Icon::Pixmap(icon_pixmap) => parse_icon_given_pixmaps(icon_pixmap, size),
            Icon::Empty => None,
        }
    }
}
impl PartialEq for Icon {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Named {
                    name: l_name,
                    theme_path: l_theme_path,
                },
                Self::Named {
                    name: r_name,
                    theme_path: r_theme_path,
                },
            ) => l_name == r_name && l_theme_path == r_theme_path,
            (Self::Empty, Self::Empty) => true,
            // THIS OPERATION IS HEAVY
            (Self::PngData(_), Self::PngData(_)) | (Self::Pixmap(_), Self::Pixmap(_)) => false,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Debug)]
pub struct RootMenu {
    #[allow(dead_code)]
    pub id: i32,
    pub submenus: Vec<MenuItem>,
}
impl RootMenu {
    pub fn from_tray_menu(tray_menu: &system_tray::menu::TrayMenu) -> Self {
        Self {
            id: tray_menu.id as i32,
            submenus: tray_menu
                .submenus
                .iter()
                .map(MenuItem::from_menu_item)
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct MenuItem {
    pub id: i32,
    pub enabled: bool,
    pub label: Option<String>,
    pub icon: Option<Icon>,
    pub menu_type: MenuType,

    pub submenu: Option<Vec<MenuItem>>,
}

impl MenuItem {
    fn from_menu_item(value: &system_tray::menu::MenuItem) -> Self {
        let id = value.id;
        let label = value.label.clone();
        let enabled = value.enabled;

        let icon = value.icon_data.clone().map(Icon::PngData).or_else(|| {
            value.icon_name.clone().map(|name| Icon::Named {
                name,
                theme_path: None,
            })
        });

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
                    system_tray::menu::ToggleType::CannotBeToggled => MenuType::Normal,
                }
            }
        };

        let submenu = if let Some("submenu") = &value.children_display.as_deref() {
            Some(value.submenu.iter().map(MenuItem::from_menu_item).collect())
        } else {
            None
        };

        Self {
            id,
            label,
            enabled,
            icon,
            menu_type,
            submenu,
        }
    }
}

#[derive(Debug)]
pub enum MenuType {
    Radio(bool),
    Check(bool),
    // should the menu wtih submenus have toggle states?
    Separator,
    Normal,
}

#[derive(Debug)]
pub struct Tray {
    pub destination: String,
    pub id: String,
    pub title: Option<String>,
    pub icon: Icon,
    pub menu_path: Option<String>,
    pub menu: Option<RootMenu>,
}

macro_rules! diff_and_update {
    ($old:expr, $new:expr) => {{
        let diff = $new != $old;
        if diff {
            $old = $new;
        }
        diff
    }};
}

impl Tray {
    pub fn update_title(&mut self, title: Option<String>) -> bool {
        diff_and_update!(self.title, title)
    }
    pub fn update_icon(&mut self, icon: Icon) -> bool {
        diff_and_update!(self.icon, icon)
    }
    pub fn update_menu(&mut self, new: Option<RootMenu>) {
        self.menu = new;
    }
}

pub fn create_tray_item(destination: String, value: &StatusNotifierItem) -> Tray {
    let id = value.id.clone();
    let title = value.title.clone();

    let icon = value
        .icon_name
        .clone()
        .filter(|icon_name| !icon_name.is_empty())
        .map(|name| Icon::Named {
            name,
            theme_path: value.icon_theme_path.clone(),
        })
        .or_else(|| value.icon_pixmap.clone().map(Icon::Pixmap))
        .unwrap_or(Icon::Empty);

    let menu_path = value.menu.clone();

    Tray {
        destination,
        id,
        title,
        icon,
        menu_path,
        menu: None,
    }
}
