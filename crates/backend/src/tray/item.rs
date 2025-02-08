use std::{cell::RefCell, collections::HashMap};

use cairo::ImageSurface;
use system_tray::item::{IconPixmap, StatusNotifierItem};

use super::icon::{
    fallback_icon, parse_icon_given_data, parse_icon_given_name, parse_icon_given_pixmaps,
    IconThemeNameOrPath,
};

#[derive(Debug, Hash, PartialEq, Eq)]
struct IconCacheKey {
    size: i32,
    t: IconCacheType,
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum IconCacheType {
    Name {
        name: String,
        theme: Option<String>,
        theme_path: Option<String>,
    },
    PngData,
    Pixmap,
}

#[derive(Debug)]
pub struct IconHandle {
    cache: RefCell<HashMap<IconCacheKey, ImageSurface>>,
    icon: Option<Icon>,
}
impl IconHandle {
    fn new(icon: Option<Icon>) -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
            icon,
        }
    }
    pub fn draw_icon(
        &self,
        size: i32,
        theme: Option<&str>,
        theme_path: Option<&str>,
    ) -> ImageSurface {
        let Some(icon) = self.icon.as_ref() else {
            return ImageSurface::create(cairo::Format::ARgb32, size, size).unwrap();
        };

        // cache
        let cache_key = IconCacheKey {
            size,
            t: match icon {
                Icon::Named(name) => IconCacheType::Name {
                    name: name.clone(),
                    theme: theme.map(ToString::to_string),
                    theme_path: theme_path.map(ToString::to_string),
                },
                Icon::PngData(_) => IconCacheType::PngData,
                Icon::Pixmap(_) => IconCacheType::Pixmap,
            },
        };
        if let Some(cache) = self.cache.borrow().get(&cache_key).cloned() {
            return cache;
        }

        if let Some(content) = icon.draw_icon(size, theme, theme_path) {
            self.cache.borrow_mut().insert(cache_key, content.clone());
            return content;
        }

        fallback_icon(size, theme)
            .unwrap_or(ImageSurface::create(cairo::Format::ARgb32, size, size).unwrap())
    }
}

#[derive(Debug)]
pub enum Icon {
    Named(String),
    PngData(Vec<u8>),
    Pixmap(Vec<IconPixmap>),
}
impl Icon {
    pub fn draw_icon(
        &self,
        size: i32,
        theme: Option<&str>,
        theme_path: Option<&str>,
    ) -> Option<ImageSurface> {
        match self {
            Icon::Named(name) => {
                let theme_or_path = theme_path
                    .filter(|path| !path.is_empty())
                    .map(IconThemeNameOrPath::Path)
                    .unwrap_or_else(|| IconThemeNameOrPath::Name(theme));
                parse_icon_given_name(name, size, theme_or_path)
            }
            Icon::PngData(items) => parse_icon_given_data(items, size),
            Icon::Pixmap(icon_pixmap) => parse_icon_given_pixmaps(icon_pixmap, size),
        }
    }
}
impl PartialEq for Icon {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Named(l0), Self::Named(r0)) => l0 == r0,
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
    pub(super) fn from_tray_menu(tray_menu: system_tray::menu::TrayMenu) -> Self {
        Self {
            id: tray_menu.id as i32,
            submenus: tray_menu
                .submenus
                .into_iter()
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
    pub icon: Option<IconHandle>,
    pub menu_type: MenuType,

    pub submenu: Option<Vec<MenuItem>>,
}

impl MenuItem {
    fn from_menu_item(value: system_tray::menu::MenuItem) -> Self {
        let system_tray::menu::MenuItem {
            id,
            menu_type,
            label,
            enabled,
            icon_name,
            icon_data,
            toggle_type,
            toggle_state,
            children_display,
            submenu,
            ..
            // shortcut,
            // visible,
            // disposition,
        } = value;

        let icon = icon_data
            .map(Icon::PngData)
            .or_else(|| icon_name.map(Icon::Named))
            .map(Some)
            .map(IconHandle::new);

        let menu_type = match menu_type {
            system_tray::menu::MenuType::Separator => MenuType::Separator,
            system_tray::menu::MenuType::Standard => {
                match toggle_type {
                    system_tray::menu::ToggleType::Checkmark => {
                        MenuType::Check(match toggle_state {
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
                        MenuType::Radio(match toggle_state {
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

        let submenu = if let Some("submenu") = children_display.as_deref() {
            Some(submenu.into_iter().map(MenuItem::from_menu_item).collect())
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
    pub id: String,
    pub title: Option<String>,
    pub icon: IconHandle,
    pub icon_theme_path: Option<String>,
    pub menu_path: Option<String>,
    pub menu: Option<RootMenu>,
}

impl Tray {
    pub(super) fn new(value: StatusNotifierItem) -> Self {
        let StatusNotifierItem {
            id,
            title,
            icon_theme_path,
            icon_name,
            icon_pixmap,
            menu,
            ..
            // category,
            // status,
            // window_id,
            // overlay_icon_name,
            // overlay_icon_pixmap,
            // attention_icon_name,
            // attention_icon_pixmap,
            // attention_movie_name,
            // tool_tip,
            // item_is_menu,
        } = value;

        let icon = icon_name
            .filter(|icon_name| !icon_name.is_empty())
            .map(Icon::Named)
            .or_else(|| icon_pixmap.map(Icon::Pixmap));

        let icon = IconHandle::new(icon);

        let menu_path = menu;

        Tray {
            id,
            title,
            icon,
            menu_path,
            menu: None,
            icon_theme_path,
        }
    }
    pub(super) fn update_title(&mut self, title: Option<String>) -> bool {
        if self.title != title {
            self.title = title;
            true
        } else {
            false
        }
    }
    pub(super) fn update_icon(&mut self, icon: Option<Icon>) -> bool {
        if self.icon.icon != icon {
            self.icon = IconHandle::new(icon);
            true
        } else {
            false
        }
    }
    pub(super) fn update_menu(&mut self, new: RootMenu) {
        self.menu.replace(new);
    }
}
