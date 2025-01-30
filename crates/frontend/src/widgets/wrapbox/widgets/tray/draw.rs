use std::f64::consts::PI;

use cairo::{Context, ImageSurface};

use config::widgets::wrapbox::tray::{HeaderDrawConfig, MenuDrawConfig};
use gdk::prelude::GdkCairoContextExt;
use util::{
    draw::new_surface,
    rgba_to_color,
    text::{draw_text, TextConfig},
    Z,
};

use super::item::{MenuItem, MenuState, MenuType, Tray};

pub struct HeaderDrawArg<'a> {
    text_conf: TextConfig<'a>,
}
impl<'a> HeaderDrawArg<'a> {
    pub fn create_from_config(draw_config: &'a HeaderDrawConfig) -> Self {
        let text_conf = TextConfig::new(
            None,
            None,
            rgba_to_color(draw_config.text_color),
            draw_config.font_pixel_height,
        );

        Self { text_conf }
    }
    pub fn draw_header(&self, tray: &Tray) -> ImageSurface {
        if !tray.is_open {
            return tray.icon.clone();
        }

        let Some(title) = tray.title.as_ref().filter(|title| !title.is_empty()) else {
            return tray.icon.clone();
        };

        let text_surf = self.draw_text(title);

        static ICON_TEXT_GAP: i32 = 4;

        combine_horizonal_center(&[tray.icon.clone(), text_surf], Some(ICON_TEXT_GAP))
    }
    fn draw_text(&self, text: &str) -> ImageSurface {
        draw_text(text, self.text_conf).to_image_surface()
    }
}

enum MenuItemDrawResult {
    Item(ImageSurface),
    Separator(i32),
}
impl MenuItemDrawResult {
    fn get_size(&self) -> (i32, i32) {
        match self {
            MenuItemDrawResult::Item(surf) => (surf.width(), surf.height()),
            MenuItemDrawResult::Separator(height) => (0, *height),
        }
    }
}

static GAP_BETWEEN_MARKER_AND_TEXT: i32 = 5;

// TODO: ICON FOR MENU ITEM
pub struct MenuDrawArg<'a> {
    draw_config: &'a MenuDrawConfig,
    text_conf: TextConfig<'a>,
}
impl<'a> MenuDrawArg<'a> {
    pub fn create_from_config(draw_config: &'a MenuDrawConfig) -> Self {
        let text_conf = TextConfig::new(
            None,
            None,
            rgba_to_color(draw_config.text_color),
            draw_config.font_pixel_height,
        );

        Self {
            draw_config,
            text_conf,
        }
    }

    pub fn draw_menu(&self, menu: &[MenuItem], menu_state: &MenuState) -> (ImageSurface, Vec<f64>) {
        // this should be in config, or?
        static MENU_ITEM_BORDER_WIDTH: i32 = 4;

        let last_menu_index = menu.len() - 1;
        let mut max_width = 0;
        let mut total_height = 0;
        let menu_draw_res: Vec<MenuItemDrawResult> = menu
            .iter()
            .enumerate()
            .map(|(index, item)| {
                // current_item
                let menu_res = self.draw_menu_item(item);

                // count size
                let size = menu_res.get_size();
                max_width = max_width.max(size.0);
                total_height += size.1;

                // count in menu border
                if index != last_menu_index {
                    total_height += MENU_ITEM_BORDER_WIDTH;
                }

                menu_res
            })
            .collect();

        // context and surface
        let size = (
            max_width + MENU_ITEM_BORDER_WIDTH * 2,
            total_height + MENU_ITEM_BORDER_WIDTH * 2,
        );
        let surf = new_surface(size);
        let ctx = Context::new(&surf).unwrap();
        ctx.set_source_color(&self.draw_config.border_color);
        ctx.set_line_width(MENU_ITEM_BORDER_WIDTH as f64);

        // outline of the menu
        let half_line = MENU_ITEM_BORDER_WIDTH as f64 / 2.;
        ctx.rectangle(
            half_line,
            half_line,
            half_line + max_width as f64,
            half_line + total_height as f64,
        );
        ctx.stroke().unwrap();
        ctx.translate(half_line, half_line);

        // menu item draw func
        let draw_menu_border = || {
            // draw a bottom border line
            ctx.set_source_color(&self.draw_config.border_color);
            ctx.move_to(Z, half_line);
            ctx.rel_line_to(max_width as f64, Z);
            ctx.stroke().unwrap();

            // translate
            ctx.translate(Z, MENU_ITEM_BORDER_WIDTH as f64);
        };
        let draw_menu_img = |index: usize, img: ImageSurface| {
            let height = img.height() as f64;

            ctx.set_source_surface(&img, Z, Z).unwrap();
            ctx.paint().unwrap();

            // menu state
            let menu_item = &menu[index];
            if !menu_item.enabled {
                // not enable
                ctx.save().unwrap();
                ctx.set_source_rgba(0., 0., 0., 0.2);
                ctx.rectangle(Z, Z, max_width as f64, height);
                ctx.fill().unwrap();
                ctx.restore().unwrap();
            } else if menu_state.is_hover(menu_item.id) {
                // hover
                ctx.save().unwrap();
                ctx.set_source_rgba(1., 1., 1., 0.2);
                ctx.rectangle(Z, Z, max_width as f64, height);
                ctx.fill().unwrap();
                ctx.restore().unwrap();
            }

            if index < last_menu_index {
                ctx.translate(Z, height);
                draw_menu_border();
            }
        };
        let draw_menu_sep = |index: usize, height: i32| {
            ctx.set_source_color(&self.draw_config.border_color);
            ctx.rectangle(Z, Z, max_width as f64, height as f64);
            ctx.fill().unwrap();

            ctx.translate(Z, height as f64);
            if index < last_menu_index {
                draw_menu_border();
            }
        };

        // y map for layout
        let mut y_map = vec![];
        let mut y_count = 0.;
        let mut count_y_map = |index, height: i32| {
            let item_height = if index == 0 {
                half_line + MENU_ITEM_BORDER_WIDTH as f64 + height as f64
            } else if index == last_menu_index {
                size.1 as f64
            } else {
                MENU_ITEM_BORDER_WIDTH as f64 + height as f64
            };

            y_count += item_height;
            y_map.push(y_count);
        };

        // iter menu item and draw
        menu_draw_res
            .into_iter()
            .enumerate()
            .for_each(|(index, res)| match res {
                MenuItemDrawResult::Item(img) => {
                    let height = img.height();
                    draw_menu_img(index, img);
                    count_y_map(index, height);
                }
                MenuItemDrawResult::Separator(height) => {
                    draw_menu_sep(index, height);
                    count_y_map(index, height);
                }
            });

        (surf, y_map)
    }

    fn draw_menu_item(&self, item: &MenuItem) -> MenuItemDrawResult {
        let mut imgs = Vec::with_capacity(4);

        // marker
        match item.menu_type {
            MenuType::Radio(state) => imgs.push(self.draw_marker_radio(state)),
            MenuType::Check(state) => imgs.push(self.draw_marker_check(state)),
            // empty marker
            MenuType::Normal => imgs.push(new_surface((
                self.draw_config.marker_size,
                self.draw_config.marker_size,
            ))),
            // do not draw anything
            MenuType::Separator => {
                return MenuItemDrawResult::Separator(self.draw_config.separator_height)
            }
        }

        // icon
        if let Some(icon) = item.icon.as_ref() {
            imgs.push(icon.clone());
        }

        // text
        if let Some(label) = item.label.as_ref() {
            imgs.push(self.draw_text(label))
        }

        // submenu marker
        if item.submenu.is_some() {
            imgs.push(self.draw_marker_parent());
        }

        // combined
        let combined = combine_horizonal_center(&imgs, Some(GAP_BETWEEN_MARKER_AND_TEXT));

        // margin
        let surf = new_surface((
            combined.width() + 2 * self.draw_config.margin[0],
            combined.height() + 2 * self.draw_config.margin[1],
        ));
        let ctx = Context::new(&surf).unwrap();
        ctx.set_source_surface(
            &combined,
            self.draw_config.margin[0] as f64,
            self.draw_config.margin[1] as f64,
        )
        .unwrap();
        ctx.paint().unwrap();

        MenuItemDrawResult::Item(surf)
    }
}

// supports: markers, text
impl MenuDrawArg<'_> {
    fn draw_text(&self, text: &str) -> ImageSurface {
        draw_text(text, self.text_conf).to_image_surface()
    }
    fn get_marker_surf_context(&self) -> (ImageSurface, Context) {
        let size = self.draw_config.marker_size;
        let color = self
            .draw_config
            .marker_color
            .unwrap_or(self.draw_config.text_color);

        let surf = new_surface((size, size));
        let ctx = Context::new(&surf).unwrap();
        ctx.set_source_color(&color);

        (surf, ctx)
    }
    fn draw_marker_radio(&self, state: bool) -> ImageSurface {
        let (surf, ctx) = self.get_marker_surf_context();

        let size = self.draw_config.marker_size;

        let center = size as f64 / 2.;
        let line_width = (size as f64 / 10.).ceil();
        let radius = center - line_width / 2.;
        ctx.set_line_width(line_width);
        ctx.arc(center, center, radius, Z, 2. * PI);
        ctx.stroke().unwrap();

        if state {
            let radius = size as f64 / 5.;
            ctx.arc(center, center, radius, Z, 2. * PI);
            ctx.fill().unwrap();
        }

        surf
    }
    fn draw_marker_check(&self, state: bool) -> ImageSurface {
        let (surf, ctx) = self.get_marker_surf_context();

        let size = self.draw_config.marker_size;

        ctx.rectangle(Z, Z, size as f64, size as f64);
        ctx.set_line_width((size as f64 / 5.).ceil());
        ctx.stroke().unwrap();

        if state {
            let inner_size = (size as f64 * 0.5).ceil();
            let start = (size as f64 - inner_size) / 2.;
            ctx.rectangle(start, start, inner_size, inner_size);
            ctx.fill().unwrap();
        }

        surf
    }
    fn draw_marker_parent(&self) -> ImageSurface {
        let (surf, ctx) = self.get_marker_surf_context();
        let size = self.draw_config.marker_size;
        ctx.move_to(Z, Z);
        ctx.line_to(size as f64, size as f64 / 2.);
        ctx.line_to(Z, size as f64);
        ctx.close_path();
        ctx.fill().unwrap();
        surf
    }
}

pub fn combine_horizonal_center(imgs: &[ImageSurface], gap: Option<i32>) -> ImageSurface {
    let last_index = imgs.len() - 1;

    let mut max_height = 0;
    let mut total_width = 0;
    imgs.iter().enumerate().for_each(|(index, img)| {
        max_height = max_height.max(img.height());
        total_width += img.width();

        // count in gap
        if index != last_index {
            if let Some(gap) = gap {
                total_width += gap;
            }
        }
    });

    let surf = new_surface((total_width, max_height));
    let ctx = Context::new(&surf).unwrap();

    imgs.iter().enumerate().for_each(|(index, img)| {
        let height = img.height();
        let y = (max_height - height) / 2;
        ctx.set_source_surface(img, Z, y as f64).unwrap();
        ctx.paint().unwrap();
        ctx.translate(img.width() as f64, Z);

        // translate for gap
        if index != last_index {
            if let Some(gap) = gap {
                ctx.translate(gap as f64, Z);
            }
        }
    });

    surf
}
