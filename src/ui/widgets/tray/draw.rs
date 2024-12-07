use std::f64::consts::PI;

use cairo::{Context, ImageSurface};
use gtk::{gdk::RGBA, pango::Layout, prelude::GdkCairoContextExt};

use crate::ui::draws::util::{draw_text_to_size, new_surface, Z};

use super::module::{MenuItem, MenuState, MenuType};

pub struct MenuDrawConfig {
    margin: [i32; 4],
    font_pixel_height: i32,
    marker_size: i32,
    separator_height: i32,
    text_color: RGBA,
    marker_color: Option<RGBA>,
}
impl Default for MenuDrawConfig {
    fn default() -> Self {
        Self {
            margin: [5; 4],
            marker_size: 16,
            font_pixel_height: 16,
            separator_height: 5,
            text_color: RGBA::BLACK,
            marker_color: None,
        }
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
    layout: Layout,
    cell_height: i32,
    marker_start_pos: (f64, f64),
    text_start_pos: (f64, f64),
}
impl<'a> MenuDrawArg<'a> {
    pub fn create_from_config(draw_config: &'a MenuDrawConfig) -> Self {
        let layout = {
            let font_size = draw_config.font_pixel_height;
            let pc = pangocairo::pango::Context::new();
            let mut desc = pc.font_description().unwrap();
            desc.set_absolute_size(font_size as f64 * 1024.);
            pc.set_font_description(Some(&desc));
            pangocairo::pango::Layout::new(&pc)
        };

        let cell_height = draw_config.font_pixel_height.max(draw_config.marker_size)
            + draw_config.margin[1]
            + draw_config.margin[3];

        let mut marker_start_y = 0;
        let mut text_start_y = 0;
        let minused = draw_config.marker_size - draw_config.font_pixel_height;

        #[allow(clippy::comparison_chain)]
        if minused > 0 {
            // marker is bigger
            text_start_y += minused / 2;
        } else if minused < 0 {
            // text is bigger
            marker_start_y += -minused / 2;
        }

        let marker_start_pos = (draw_config.margin[0] as f64, marker_start_y as f64);
        let text_start_pos = (
            draw_config.margin[0] as f64
                + draw_config.marker_size as f64
                + GAP_BETWEEN_MARKER_AND_TEXT as f64,
            text_start_y as f64,
        );

        Self {
            draw_config,
            layout,

            cell_height,
            marker_start_pos,
            text_start_pos,
        }
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

    pub fn draw_menu(&self, menu: &[MenuItem], menu_state: &MenuState) -> (ImageSurface, Vec<f64>) {
        let mut max_width = 0;
        let mut total_height = 0;
        let menu_draw_res: Vec<MenuItemDrawResult> = menu
            .iter()
            .map(|item| {
                // current_item
                let menu_res = self.draw_menu_item(item);
                let size = menu_res.get_size();
                max_width = max_width.max(size.0);
                total_height += size.1;
                menu_res
            })
            .collect();

        // this should be in config, or?
        static MENU_OUTLINE_WIDTH: i32 = 4;

        let surf = new_surface((
            max_width + MENU_OUTLINE_WIDTH * 2,
            total_height + MENU_OUTLINE_WIDTH * 2,
        ));
        let ctx = Context::new(&surf).unwrap();
        ctx.set_line_width(MENU_OUTLINE_WIDTH as f64);

        // outline of the menu
        let half_line = MENU_OUTLINE_WIDTH as f64 / 2.;
        ctx.rectangle(
            half_line,
            half_line,
            half_line + max_width as f64,
            half_line + total_height as f64,
        );
        ctx.stroke().unwrap();
        ctx.translate(half_line, half_line);

        let item_len = menu.len();
        let draw_menu_img = |index: usize, img: ImageSurface| {
            let height = img.height() as f64;

            ctx.set_source_surface(&img, Z, Z).unwrap();
            ctx.paint().unwrap();

            // hover
            let menu_item = &menu[index];
            if menu_state.is_hover(menu_item.id) {
                ctx.save().unwrap();
                ctx.set_source_rgba(1., 1., 1., 0.2);
                ctx.rectangle(Z, Z, img.width() as f64, img.height() as f64);
                ctx.fill().unwrap();
                ctx.restore().unwrap();
            }

            if index < item_len - 1 {
                // draw a separator line
                ctx.move_to(Z, height + half_line);
                ctx.rel_line_to(max_width as f64, Z);
                ctx.stroke().unwrap();

                // translate
                ctx.translate(Z, height + MENU_OUTLINE_WIDTH as f64);
            }
        };
        let draw_menu_sep = |index: usize, height: i32| {
            ctx.translate(Z, height as f64);
            if index < item_len - 1 {
                // draw a separator line
                ctx.move_to(Z, height as f64 + half_line);
                ctx.rel_line_to(max_width as f64, Z);
                ctx.stroke().unwrap();

                // translate
                ctx.translate(Z, MENU_OUTLINE_WIDTH as f64);
            }
        };

        let mut y_map = vec![];
        let mut y_count = 0.;
        let mut count_y_map = |index, height: i32| {
            if index == 0 || index == item_len - 1 {
                let item_height = half_line + MENU_OUTLINE_WIDTH as f64 + height as f64;
                y_map.push(item_height);
                y_count += item_height;
            } else {
                let item_height = 2. * MENU_OUTLINE_WIDTH as f64 + height as f64;
                y_map.push(item_height);
                y_count += item_height;
            }
        };

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

    fn draw_menu_item(&self, item: &MenuItem) -> MenuItemDrawResult {
        let mut width = self.text_start_pos.0 + self.draw_config.margin[3] as f64;

        let mut text_img = None;
        if let Some(label) = item.label.as_ref() {
            let img = self.draw_text(label);
            width += img.width() as f64;
            text_img = Some(img);
        }

        let surf = new_surface((width as i32, self.cell_height));
        let ctx = Context::new(&surf).unwrap();

        let draw_marker = |img: ImageSurface| {
            ctx.set_source_surface(img, self.marker_start_pos.0, self.marker_start_pos.1)
                .unwrap();
            ctx.paint().unwrap();
        };
        match item.menu_type {
            MenuType::Radio(state) => draw_marker(self.draw_marker_radio(state)),
            MenuType::Check(state) => draw_marker(self.draw_marker_check(state)),
            MenuType::Separator => {
                return MenuItemDrawResult::Separator(self.draw_config.separator_height)
            }
            MenuType::Normal => {}
        }

        if let Some(img) = text_img {
            ctx.set_source_surface(&img, self.text_start_pos.0, self.text_start_pos.1)
                .unwrap();
            ctx.paint().unwrap();

            // for submenu
            ctx.translate(
                self.text_start_pos.0 + img.width() as f64 + GAP_BETWEEN_MARKER_AND_TEXT as f64,
                Z,
            );
        }

        // submenu marker
        if item.submenu.is_some() {
            let img = self.draw_marker_parent();
            ctx.set_source_surface(img, Z, self.marker_start_pos.1)
                .unwrap();
            ctx.paint().unwrap();
        }

        MenuItemDrawResult::Item(surf)
    }

    pub fn draw_text(&self, text: &str) -> ImageSurface {
        draw_text_to_size(
            &self.layout,
            &self.draw_config.text_color,
            text,
            self.draw_config.font_pixel_height,
        )
    }
}
