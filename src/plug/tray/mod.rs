mod context;
mod event;

pub use context::{register_tray, unregister_tray};
pub use event::Event;

use std::{io::Cursor, path::PathBuf};

use cairo::ImageSurface;
use context::get_tray_context;
use gio::prelude::FileExt;
use gtk::{
    gdk_pixbuf::{Colorspace, Pixbuf},
    prelude::GdkCairoContextExt,
    IconLookupFlags, IconPaintable, TextDirection, PAPER_NAME_B5,
};
use system_tray::item::{IconPixmap, StatusNotifierItem};

use crate::ui::draws::util::Z;

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

        let icon = value
            .icon_name
            .filter(|icon_name| !icon_name.is_empty())
            .map(TrayIcon::Name)
            .or_else(|| {
                if let Some(icon_pix_map) = value.icon_pixmap {
                    println!("icon_pixmap: {icon_pix_map:?}");
                    Some(TrayIcon::Pixmap(icon_pix_map))
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let menu_path = value.menu;
        Self {
            id,
            title,
            icon,
            menu_path,
        }
    }
}

fn new_image_surface_from_buf(buf: Pixbuf) -> ImageSurface {
    let width = buf.width();
    let height = buf.height();
    let format = cairo::Format::ARgb32;
    let surf = ImageSurface::create(format, width, height).unwrap();
    let context = cairo::Context::new(&surf).unwrap();

    context.set_source_pixbuf(&buf, Z, Z);
    context.paint().unwrap();

    surf
}

fn scale_image_to_size(img: ImageSurface, size: i32) -> ImageSurface {
    if img.width() == size || img.height() == size {
        return img;
    }

    let format = cairo::Format::ARgb32;
    let surf = ImageSurface::create(format, size, size).unwrap();
    let context = cairo::Context::new(&surf).unwrap();
    let scale = size as f64 / img.width() as f64;
    context.scale(scale, scale);
    context.set_source_surface(&img, Z, Z);
    context.paint().unwrap();

    surf
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
    fn parse_icon_paintable(p: IconPaintable) -> Option<Pixbuf> {
        // we can do endian convert, but it's too hard
        // https://stackoverflow.com/a/10588779/21873016

        let f = p.file()?.path()?;
        Pixbuf::from_file(f.as_path()).ok()
    }
    pub fn get_icon_with_size(&self, size: i32) -> Option<ImageSurface> {
        match self {
            TrayIcon::Name(name) => {
                // backup
                let icon_paintable = get_tray_context().get_icon_theme().lookup_icon(
                    name,
                    &[],
                    size,
                    1,
                    TextDirection::Ltr,
                    IconLookupFlags::empty(),
                );
                let pixbuf = Self::parse_icon_paintable(icon_paintable)?;
                Some(scale_image_to_size(
                    new_image_surface_from_buf(pixbuf),
                    size,
                ))
            }
            TrayIcon::Data(vec) => ImageSurface::create_from_png(&mut Cursor::new(vec))
                .ok()
                .map(|img| scale_image_to_size(img, size)),
            TrayIcon::Pixmap(vec) => {
                if vec.is_empty() {
                    Self::default().get_icon_with_size(size)
                } else {
                    let pixmap = vec.last().unwrap();
                    let mut pixels = pixmap.pixels.clone();

                    // from ironbar
                    for i in (0..pixels.len()).step_by(4) {
                        let alpha = pixels[i];
                        pixels[i] = pixels[i + 1];
                        pixels[i + 1] = pixels[i + 2];
                        pixels[i + 2] = pixels[i + 3];
                        pixels[i + 3] = alpha;
                    }

                    let pixbuf = Pixbuf::from_mut_slice(
                        &mut pixels,
                        Colorspace::Rgb,
                        true,
                        8,
                        pixmap.width,
                        pixmap.height,
                        pixmap.width * 4,
                    );

                    Some(scale_image_to_size(
                        new_image_surface_from_buf(pixbuf),
                        size,
                    ))
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
