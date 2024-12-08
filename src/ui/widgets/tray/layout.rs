use cairo::{Context, ImageSurface};

use crate::{
    common::binary_search_end,
    ui::{
        draws::util::{combine_2_image_vertical_left, new_surface, Z},
        widgets::tray::draw::{MenuDrawArg, MenuDrawConfig},
    },
};

use super::{
    draw::{HeaderDrawArg, HeaderDrawConfig},
    module::{MenuItem, MenuState, RootMenu, Tray},
};

#[derive(Debug)]
pub enum HoveringItem {
    TrayIcon,
    MenuItem(i32),
}

#[derive(Default)]
struct TrayHeadLayout {
    // icon should always be at 0,0
    content_size: (i32, i32),
}
impl TrayHeadLayout {
    fn draw_and_create(tray: &Tray) -> (ImageSurface, Self) {
        let draw_config = HeaderDrawConfig::default();

        let draw_arg = HeaderDrawArg::create_from_config(&draw_config);

        let img = draw_arg.draw_header(tray);
        let content_size = (img.width(), img.height());

        (img, Self { content_size })
    }
    fn get_hovering(&self, pos: (f64, f64)) -> bool {
        pos.0 >= Z
            && pos.0 < self.content_size.0 as f64
            && pos.1 >= Z
            && pos.1 < self.content_size.1 as f64
    }
}

#[derive(Debug)]
struct MenuCol {
    height_range: Vec<f64>,
    id_vec: Vec<i32>,
}
impl MenuCol {
    fn draw_and_create_from_root_menu(
        menu_items: &[MenuItem],
        state: &MenuState,
        menu_arg: &mut MenuDrawArg,
    ) -> Vec<(ImageSurface, Self)> {
        let (surf, height_range) = menu_arg.draw_menu(menu_items, state);

        let mut next_col = None;

        let id_vec: Vec<i32> = menu_items
            .iter()
            .map(|item| {
                // check next col
                if let Some(submenu) = &item.submenu {
                    if state.is_open(item.id) {
                        next_col = Some(submenu);
                    }
                }

                item.id
            })
            .collect();

        let mut res = vec![(
            surf,
            Self {
                height_range,
                id_vec,
            },
        )];

        if let Some(next_col) = next_col {
            let next_col = Self::draw_and_create_from_root_menu(next_col, state, menu_arg);
            res.extend(next_col);
        }

        res
    }
    fn get_hovering(&self, pos: (f64, f64)) -> Option<i32> {
        println!("self: {:?}, pos: {pos:?}", self);

        let row_index = binary_search_end(&self.height_range, pos.1);

        println!("row index: {row_index}");

        if row_index == -1 {
            None
        } else {
            Some(self.id_vec[row_index as usize])
        }
    }
}

#[derive(Debug)]
struct MenuLayout {
    // end pixel index of each col
    menu_each_col_x_end: Vec<i32>,
    // same index of `menu_each_col_x_end`
    menu_cols: Vec<MenuCol>,
}
impl MenuLayout {
    fn draw_and_create(root_menu: &RootMenu, state: &MenuState) -> (ImageSurface, Self) {
        let config = MenuDrawConfig::default();
        let mut menu_arg = MenuDrawArg::create_from_config(&config);

        let cols =
            MenuCol::draw_and_create_from_root_menu(&root_menu.submenus, state, &mut menu_arg);

        drop(menu_arg);

        let mut max_height = 0;
        let mut menu_each_col_x_end = vec![];
        let mut width_count = 0;

        cols.iter().for_each(|(img, _)| {
            max_height = max_height.max(img.height());
            width_count += img.width();
            menu_each_col_x_end.push(width_count);
        });

        let surf = new_surface((width_count, max_height));
        let ctx = Context::new(&surf).unwrap();

        let menu_cols = cols
            .into_iter()
            .map(|(img, col)| {
                let width = img.width();
                ctx.set_source_surface(img, Z, Z).unwrap();
                ctx.paint().unwrap();
                ctx.translate(width as f64, Z);

                col
            })
            .collect();

        (
            surf,
            Self {
                menu_each_col_x_end,
                menu_cols,
            },
        )
    }
    fn get_hovering(&self, pos: (f64, f64)) -> Option<i32> {
        let col_index = binary_search_end(&self.menu_each_col_x_end, pos.0 as i32);
        if col_index == -1 {
            None
        } else {
            let col_index = col_index as usize;
            let new_pos_width = if col_index == 0 {
                0.
            } else {
                pos.0 - self.menu_each_col_x_end[col_index - 1] as f64
            };
            self.menu_cols[col_index].get_hovering((new_pos_width, pos.1))
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

        macro_rules! done_with_only_header {
            ($tray:expr, $header_img:expr, $header_layout:expr) => {
                $tray.content = $header_img;
                $tray.layout = TrayLayout {
                    tray_head_layout: $header_layout,
                    menu_layout: None,
                };
            };
        }

        if !tray.is_open {
            done_with_only_header!(tray, header_img, header_layout);
            return;
        }

        let Some((menu_img, menu_layout)) = tray
            .menu
            .as_ref()
            .map(|(root_menu, menu_state)| MenuLayout::draw_and_create(root_menu, menu_state))
        else {
            done_with_only_header!(tray, header_img, header_layout);
            return;
        };

        static GAP_HEADER_MENU: i32 = 6;

        // combine header and menu
        tray.content = combine_2_image_vertical_left(&header_img, &menu_img, Some(GAP_HEADER_MENU));
        tray.layout = TrayLayout {
            tray_head_layout: header_layout,
            menu_layout: Some(menu_layout),
        };
    }

    pub fn get_hovering(&self, pos: (f64, f64)) -> Option<HoveringItem> {
        if pos.1 < self.tray_head_layout.content_size.1 as f64 {
            self.tray_head_layout
                .get_hovering(pos)
                .then_some(HoveringItem::TrayIcon)
        } else {
            self.menu_layout.as_ref().and_then(|menu_layout| {
                menu_layout
                    .get_hovering((pos.0, pos.1 - self.tray_head_layout.content_size.1 as f64))
                    .map(HoveringItem::MenuItem)
            })
        }
    }
}
