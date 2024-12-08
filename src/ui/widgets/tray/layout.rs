use cairo::{Context, ImageSurface};

use crate::{
    common::binary_search_end,
    config::widgets::wrapbox::tray::{
        HeaderDrawConfig, HeaderMenuAlign, HeaderMenuStack, MenuDrawConfig,
    },
    ui::{
        draws::util::{combine_vertcal, new_surface, Z},
        widgets::tray::draw::MenuDrawArg,
    },
};

use super::{
    draw::HeaderDrawArg,
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
    header_height: i32,
}
impl TrayHeadLayout {
    fn draw_and_create(tray: &Tray, header_draw_config: &HeaderDrawConfig) -> (ImageSurface, Self) {
        let draw_arg = HeaderDrawArg::create_from_config(header_draw_config);

        let img = draw_arg.draw_header(tray);
        // let content_size = (img.width(), img.height());
        let header_height = img.height();

        (img, Self { header_height })
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
        let row_index = binary_search_end(&self.height_range, pos.1);

        if row_index == -1 {
            None
        } else {
            Some(self.id_vec[row_index as usize])
        }
    }
}

#[derive(Debug)]
struct MenuLayout {
    menu_size: (i32, i32),
    // end pixel index of each col
    menu_each_col_x_end: Vec<i32>,
    // same index of `menu_each_col_x_end`
    menu_cols: Vec<MenuCol>,
}
impl MenuLayout {
    fn draw_and_create(
        root_menu: &RootMenu,
        state: &MenuState,
        menu_draw_config: &MenuDrawConfig,
    ) -> (ImageSurface, Self) {
        let mut menu_arg = MenuDrawArg::create_from_config(menu_draw_config);

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

        let menu_size = (surf.width(), surf.height());
        (
            surf,
            Self {
                menu_each_col_x_end,
                menu_cols,
                menu_size,
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

static GAP_HEADER_MENU: i32 = 6;

#[derive(Default)]
pub struct TrayLayout {
    total_size: (i32, i32),

    header_menu_stack: HeaderMenuStack,
    header_menu_align: HeaderMenuAlign,

    tray_head_layout: TrayHeadLayout,
    menu_layout: Option<MenuLayout>,
}
impl TrayLayout {
    pub fn draw_and_create(tray: &mut Tray) {
        let tray_config = &tray.get_module().config;

        let (header_img, header_layout) =
            TrayHeadLayout::draw_and_create(tray, &tray_config.header_draw_config);

        macro_rules! done_with_only_header {
            ($tray:expr, $tray_config:expr, $header_img:expr, $header_layout:expr) => {
                let header_menu_stack = $tray_config.header_menu_stack.clone();
                let header_menu_align = $tray_config.header_menu_align.clone();
                let total_size = ($header_img.width(), $header_img.height());
                $tray.content = $header_img;
                $tray.layout = TrayLayout {
                    tray_head_layout: $header_layout,
                    menu_layout: None,

                    total_size,

                    header_menu_stack,
                    header_menu_align,
                };
            };
        }

        if !tray.is_open {
            done_with_only_header!(tray, tray_config, header_img, header_layout);
            return;
        }

        let Some((menu_img, menu_layout)) = tray.menu.as_ref().map(|(root_menu, menu_state)| {
            MenuLayout::draw_and_create(root_menu, menu_state, &tray_config.menu_draw_config)
        }) else {
            done_with_only_header!(tray, tray_config, header_img, header_layout);
            return;
        };

        // combine header and menu
        let imgs = match tray_config.header_menu_stack {
            HeaderMenuStack::HeaderTop => [header_img, menu_img],
            HeaderMenuStack::MenuTop => [menu_img, header_img],
        };
        let combined = combine_vertcal(
            &imgs,
            Some(GAP_HEADER_MENU),
            tray_config.header_menu_align.is_left(),
        );

        let total_size = (combined.width(), combined.height());
        let header_menu_stack = tray_config.header_menu_stack.clone();
        let header_menu_align = tray_config.header_menu_align.clone();

        tray.content = combined;
        tray.layout = TrayLayout {
            tray_head_layout: header_layout,
            menu_layout: Some(menu_layout),

            total_size,
            header_menu_stack,
            header_menu_align,
        };
    }

    pub fn get_hovering(&self, pos: (f64, f64)) -> Option<HoveringItem> {
        if pos.0 < Z
            && pos.0 > self.total_size.0 as f64
            && pos.1 < Z
            && pos.1 > self.total_size.1 as f64
        {
            return None;
        }

        let get_menu_x_when_at_left = || pos.0;
        let get_menu_x_when_at_right =
            || pos.0 - (self.total_size.0 - self.menu_layout.as_ref().unwrap().menu_size.0) as f64;

        let get_menu_y_when_at_top = || pos.1;
        let get_menu_y_when_at_bottom =
            || pos.1 - self.tray_head_layout.header_height as f64 - GAP_HEADER_MENU as f64;

        macro_rules! stack {
            (header $self:expr, $pos:expr, $menu_x:expr, $menu_y:expr) => {{
                let header_height = self.tray_head_layout.header_height as f64;
                if $pos.1 < header_height {
                    Some(HoveringItem::TrayIcon)
                } else if let Some(layout) = &self.menu_layout {
                    layout
                        .get_hovering(($menu_x, $menu_y))
                        .map(HoveringItem::MenuItem)
                } else {
                    None
                }
            }};
            (bottom $self:expr, $pos:expr, $menu_x:expr, $menu_y:expr) => {{
                if let Some(layout) = &$self
                    .menu_layout
                    .as_ref()
                    .filter(|layout| $pos.1 < layout.menu_size.1 as f64)
                {
                    layout
                        .get_hovering(($menu_x, $menu_y))
                        .map(HoveringItem::MenuItem)
                } else {
                    Some(HoveringItem::TrayIcon)
                }
            }};
        }

        match (&self.header_menu_stack, &self.header_menu_align) {
            (HeaderMenuStack::HeaderTop, HeaderMenuAlign::Left) => {
                stack!(header self, pos, get_menu_x_when_at_left(), get_menu_y_when_at_bottom())
            }
            (HeaderMenuStack::HeaderTop, HeaderMenuAlign::Right) => {
                stack!(header self, pos, get_menu_x_when_at_right(), get_menu_y_when_at_bottom())
            }
            (HeaderMenuStack::MenuTop, HeaderMenuAlign::Left) => {
                stack!(bottom self, pos, get_menu_x_when_at_left(), get_menu_y_when_at_top())
            }
            (HeaderMenuStack::MenuTop, HeaderMenuAlign::Right) => {
                stack!(bottom self, pos, get_menu_x_when_at_right(), get_menu_y_when_at_top())
            }
        }
    }
}
