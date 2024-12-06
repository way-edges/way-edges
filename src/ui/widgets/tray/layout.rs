use std::ops::Deref;

use cairo::{Format, ImageSurface};

use crate::{
    common::binary_search_end,
    ui::{
        draws::util::{draw_text, Z},
        widgets::tray::draw::{MenuDrawArg, MenuDrawConfig},
    },
};

use super::module::{MenuItem, MenuState, RootMenu, Tray};

enum ClickedItem {
    TrayIcon,
    MenuItem(i32),
}

#[derive(Default)]
struct TrayHeadLayout {
    size: (i32, i32),
    // x, y
    // icon_range: [[f64; 2]; 2],
    icon_width: i32,
}
impl TrayHeadLayout {
    fn draw_and_create(tray: &Tray) -> (ImageSurface, Self) {
        let size = (tray.icon.width(), tray.icon.height());
        // let icon_range = [[0., size.0 as f64], [0., size.1 as f64]];

        let mut icon_with_text = None;

        if tray.is_open {
            if let Some(title) = tray.title.as_ref().filter(|title| !title.is_empty()) {
                // icon and title
                // NOTE: ASSUME TITLE IS NOT EMPTY
                let layout = {
                    let pc = pangocairo::pango::Context::new();
                    let mut desc = pc.font_description().unwrap();
                    desc.set_absolute_size((size.1 << 10) as f64);
                    pc.set_font_description(Some(&desc));
                    let layout = pangocairo::pango::Layout::new(&pc);
                    layout.set_text(title);
                    layout
                };

                let text_size = layout.pixel_size();
                let surf =
                    ImageSurface::create(Format::ARgb32, size.0 + text_size.0, size.1).unwrap();
                let ctx = cairo::Context::new(&surf).unwrap();

                // draw icon
                ctx.set_source_surface(&tray.icon, Z, Z).unwrap();
                ctx.paint().unwrap();
                ctx.translate(size.0 as f64, size.1 as f64);

                // draw text
                ctx.set_antialias(cairo::Antialias::None);
                pangocairo::functions::show_layout(&ctx, &layout);
                icon_with_text = Some(surf);
            }
        }

        (
            icon_with_text.unwrap_or(tray.icon.clone()),
            Self {
                size,
                icon_width: size.0,
            },
        )
    }
    fn get_clicked(&self, pos: (f64, f64)) -> bool {
        // pos.0 >= self.icon_range[0][0]
        //     && pos.0 < self.icon_range[0][1]
        //     && pos.1 >= self.icon_range[1][0]
        //     && pos.1 < self.icon_range[1][1]
        pos.0 >= Z && pos.0 < self.icon_width as f64 && pos.1 >= Z && pos.1 < self.size.1 as f64
    }
}

struct MenuCol {
    height_range: Vec<f64>,
    id_vec: Vec<i32>,
}
impl MenuCol {
    fn draw_and_create(
        menu_items: &Vec<MenuItem>,
        state: &MenuState,
        menu_arg: &mut MenuDrawArg,
    ) -> Vec<(ImageSurface, Self)> {
        static GAP_BETWEEN_MARKER_AND_TEXT: i32 = 5;

        let next_col = None;
        let mut max_text_width = 0;
        let mut total_height = 0;
        let text_imgs: Vec<Option<ImageSurface>> = menu_items
            .iter()
            .map(|menu| {
                menu.label.map(|label| {
                    let text_img = menu_arg.draw_text(&label);
                    max_text_width = max_text_width.max(text_img.width());
                    total_height += text_img.height();
                    text_img
                })
            })
            .collect();
    }
    fn get_clicked(&self, pos: (f64, f64)) -> Option<i32> {
        let row_index = binary_search_end(&self.height_range, pos.1);
        if row_index == -1 {
            None
        } else {
            Some(self.id_vec[row_index as usize])
        }
    }
}

struct MenuLayout {
    size: (i32, i32),
    // end pixel index of each col
    menu_each_col_x_end: Vec<f64>,
    // same index of `menu_each_col_x_end`
    menu_cols: Vec<MenuCol>,
}
impl MenuLayout {
    fn draw_and_create(root_menu: &RootMenu, state: &MenuState) -> (ImageSurface, Self) {
        let config = MenuDrawConfig::default();
        let mut menu_arg = MenuDrawArg::create_from_config(&config);

        let cols = MenuCol::draw_and_create(&root_menu.submenus, state, &mut menu_arg);

        let max_height = 0;
        let total_width = 0;
        let menu_each_col_x_end = vec![];
    }
    fn get_clicked(&self, pos: (f64, f64)) -> Option<i32> {
        let col_index = binary_search_end(&self.menu_each_col_x_end, pos.0);
        if col_index == -1 {
            None
        } else {
            let col_index = col_index as usize;
            let new_pos_width = if col_index == 0 {
                0.
            } else {
                pos.0 - self.menu_each_col_x_end[col_index - 1]
            };
            self.menu_cols[col_index].get_clicked((new_pos_width, pos.1))
        }
    }
}

#[derive(Default)]
pub struct TrayLayout {
    tray_head_layout: TrayHeadLayout,
    menu_layout: Option<MenuLayout>,
}
impl TrayLayout {
    pub fn draw_and_create(tray: &mut Tray) {
        let (header_img, header_layout) = TrayHeadLayout::draw_and_create(tray);
        let Some((menu_img, menu_layout)) = tray
            .menu
            .as_ref()
            .map(|(root_menu, menu_state)| MenuLayout::draw_and_create(root_menu, menu_state))
        else {
            tray.content = header_img;
            tray.layout = TrayLayout {
                tray_head_layout: header_layout,
                menu_layout: None,
            };
            return;
        };

        // combine header and menu
        let header_size = header_layout.size;
        let menu_size = menu_layout.size;
        let surf = ImageSurface::create(
            Format::ARgb32,
            header_size.0.max(menu_size.0),
            header_size.1 + menu_size.1,
        )
        .unwrap();
        let ctx = cairo::Context::new(&surf).unwrap();
        ctx.set_source_surface(header_img, Z, Z).unwrap();
        ctx.paint().unwrap();
        ctx.translate(Z, header_size.1 as f64);
        ctx.set_source_surface(menu_img, Z, Z).unwrap();
        ctx.paint().unwrap();

        tray.content = surf;
        tray.layout = TrayLayout {
            tray_head_layout: header_layout,
            menu_layout: Some(menu_layout),
        };
    }

    pub fn get_clicked(&self, pos: (f64, f64)) -> Option<ClickedItem> {
        if pos.1 < self.tray_head_layout.size.1 as f64 {
            self.tray_head_layout
                .get_clicked(pos)
                .then_some(ClickedItem::TrayIcon)
        } else {
            self.menu_layout.as_ref().and_then(|menu_layout| {
                menu_layout
                    .get_clicked((pos.0, pos.1 - self.tray_head_layout.size.1 as f64))
                    .map(ClickedItem::MenuItem)
            })
        }

        // let max_size = (
        //     self.tray_head_layout.size.0.max(self.menu_layout.size.0) as f64,
        //     (self.tray_head_layout.size.1 + self.menu_layout.size.1) as f64,
        // );
        // if pos.1 < 0. || pos.0 < 0. || pos.0 > max_size.0 || pos.1 > max_size.1 {
        //     return None;
        // };
        //
        // if pos.1 < self.tray_head_layout.size.1 as f64 {
        //     self.tray_head_layout
        //         .get_clicked(pos)
        //         .then_some(ClickedItem::TrayIcon)
        // } else {
        //     self.menu_layout
        //         .get_clicked((pos.0, pos.1 - self.tray_head_layout.size.1 as f64))
        //         .map(ClickedItem::MenuItem)
        // }
    }
}
