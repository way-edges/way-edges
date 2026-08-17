use std::collections::HashMap;

use backend::dock::icons::IconKey;
use cairo::{Context, ImageSurface, LinearGradient, Rectangle, SurfaceType};
use config::def::widgets::dock::{DockConfig, ShowTitles};
use cosmic_text::{CacheKey, Color, FontSystem, SwashCache};
use util::color::cairo_set_color;

use super::layout::DockLayout;

fn draw_rounded_rect(
    cr: &Context,
    rect: &Rectangle,
    radius: f64,
    border_width: f64,
    border_color: Color,
    bg_color: Color,
) {
    let (x, y, w, h) = (rect.x(), rect.y(), rect.width(), rect.height());

    let draw_path = |x: f64, y: f64, w: f64, h: f64, r: f64| {
        let degrees = std::f64::consts::PI / 180.0;
        cr.new_sub_path();
        cr.arc(x + w - r, y + r, r, -90.0 * degrees, 0.0 * degrees);
        cr.arc(x + w - r, y + h - r, r, 0.0 * degrees, 90.0 * degrees);
        cr.arc(x + r, y + h - r, r, 90.0 * degrees, 180.0 * degrees);
        cr.arc(x + r, y + r, r, 180.0 * degrees, 270.0 * degrees);
        cr.close_path();
    };

    draw_path(x, y, w, h, radius);
    cairo_set_color(&cr, bg_color);
    cr.fill().unwrap();

    if border_width > 0.0 {
        let half_border = border_width / 2.0;
        draw_path(
            x + half_border,
            y + half_border,
            w - border_width,
            h - border_width,
            radius - half_border,
        );
        cr.set_line_width(border_width);
        cairo_set_color(&cr, border_color);
        cr.stroke().unwrap();
    } else {
        cr.new_path();
    }
}

fn paint_text(
    cr: &Context,
    text: &str,
    rect: &Rectangle,
    color: Color,
    font_size: i32,
    line_height: f64,
    font_family: &cosmic_text::Family,
    wrap: bool,
    align_center: bool,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut HashMap<CacheKey, ImageSurface>,
) -> () {
    let (x, y, w_limit, h_limit) = (rect.x(), rect.y(), rect.width(), rect.height());

    if text.is_empty() || w_limit <= 0.0 {
        return;
    }

    cr.save().unwrap();
    cr.rectangle(x, y, w_limit, h_limit);
    cr.clip();

    let font_height = font_size as f64 * line_height;
    let mut buffer = cosmic_text::Buffer::new(
        font_system,
        cosmic_text::Metrics::new(font_size as f32, font_height as f32),
    );

    let buffer_width = if wrap { Some(w_limit as f32) } else { None };
    buffer.set_size(font_system, buffer_width, Some(h_limit as f32));

    let attrs = cosmic_text::Attrs::new().family(*font_family);
    buffer.set_text(font_system, text, &attrs, cosmic_text::Shaping::Advanced);
    buffer.shape_until_scroll(font_system, true);

    let max_lines = if wrap {
        ((h_limit / font_height).round() as usize).max(1)
    } else {
        1
    };

    let visible_lines = buffer.layout_runs().count().min(max_lines);
    if visible_lines > 0 {
        let total_text_height = visible_lines as f64 * font_height;
        let center_offset_y = (h_limit - total_text_height) / 2.0;

        for run in buffer.layout_runs().take(visible_lines) {
            let center_offset_x = if align_center {
                (w_limit - run.line_w as f64) / 2.0
            } else {
                0.0
            };

            // if text is trucated, then fade the text color
            let is_truncated = run.line_w as f64 > w_limit;

            if is_truncated {
                let fade_width = 30.0_f64.min(w_limit * 0.3);
                let gradient = LinearGradient::new(x, 0.0, x + w_limit, 0.0);
                let r = color.r() as f64 / 255.0;
                let g = color.g() as f64 / 255.0;
                let b = color.b() as f64 / 255.0;
                let a = color.a() as f64 / 255.0;
                if align_center {
                    // if text center aligned, fade both sides
                    gradient.add_color_stop_rgba(0.0, r, g, b, 0.0);
                    gradient.add_color_stop_rgba(fade_width / w_limit, r, g, b, a);
                    gradient.add_color_stop_rgba((w_limit - fade_width) / w_limit, r, g, b, a);
                    gradient.add_color_stop_rgba(1.0, r, g, b, 0.0);
                } else {
                    // else fade to right
                    gradient.add_color_stop_rgba(0.0, r, g, b, a);
                    gradient.add_color_stop_rgba((w_limit - fade_width) / w_limit, r, g, b, a);
                    gradient.add_color_stop_rgba(1.0, r, g, b, 0.0);
                }
                cr.set_source(&gradient).unwrap();
            } else {
                // no fade when full text
                cairo_set_color(cr, color);
            }

            for glyph in run.glyphs.iter() {
                let line_start_x = (x + center_offset_x) as f32;
                let line_start_y = (y + center_offset_y) as f32 + run.line_y;

                let physical_glyph = glyph.physical((line_start_x, line_start_y), 1.0);
                let Some(image) = swash_cache.get_image(font_system, physical_glyph.cache_key)
                else {
                    continue;
                };

                if image.data.is_empty() || image.content != cosmic_text::SwashContent::Mask {
                    continue;
                }

                let gx = (physical_glyph.x + image.placement.left) as f64;
                let gy = (physical_glyph.y - image.placement.top) as f64;

                let glyph_surface =
                    glyph_cache
                        .entry(physical_glyph.cache_key)
                        .or_insert_with(|| {
                            let w_i32 = image.placement.width as i32;
                            let h_i32 = image.placement.height as i32;

                            let mut surface = ImageSurface::create(cairo::Format::A8, w_i32, h_i32)
                                .expect("Falha ao alocar superfície do glifo");

                            {
                                let stride = surface.stride() as usize;
                                let mut data = surface.data().unwrap();
                                let src = &image.data;
                                let w = image.placement.width as usize;

                                for row in 0..(image.placement.height as usize) {
                                    let src_start = row * w;
                                    let dest_start = row * stride;
                                    data[dest_start..dest_start + w]
                                        .copy_from_slice(&src[src_start..src_start + w]);
                                }
                            }
                            surface.mark_dirty();
                            surface
                        });

                cr.mask_surface(glyph_surface, gx.round(), gy.round())
                    .unwrap();
            }
        }
    }
    cr.restore().unwrap();
}

pub fn paint(
    surface: &ImageSurface,
    layout: &DockLayout,
    config: &DockConfig,
    icon_surface_cache: &HashMap<IconKey, ImageSurface>,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut HashMap<CacheKey, ImageSurface>,
) {
    let is_vertical = layout.is_vertical;

    let wk_font_size = config.workspaces.font_size;
    let wk_show_titles = config.workspaces.show_titles;

    let win_show_titles = config.windows.show_titles;
    let win_title_width = config.windows.title_width as f64;
    let win_wrap_titles = config.windows.wrap_titles;
    let win_font_size = config.windows.font_size;
    let win_line_height = config.windows.line_height;

    let icon_size = config.windows.icon_size as f64;
    let has_icon = win_show_titles != ShowTitles::Only;

    let cr = Context::new(surface).expect("Failed to create Cairo context");

    draw_rounded_rect(
        &cr,
        &layout.rect,
        config.border_radius,
        config.border_width,
        config.border_color,
        config.color,
    );

    for wksp in &layout.workspaces {
        let (wk_fg_color, wk_bg_color, wk_border_color) = if wksp.is_focused {
            (
                config.workspaces.active_fg_color,
                config.workspaces.active_bg_color,
                config.workspaces.active_border_color,
            )
        } else {
            (
                config.workspaces.fg_color,
                config.workspaces.bg_color,
                config.workspaces.border_color,
            )
        };

        if wksp.separator_rect.width() > 0.0 {
            draw_rounded_rect(
                &cr,
                &wksp.separator_rect,
                config.workspaces.separator_radius,
                0.0,
                config.workspaces.separator_color,
                config.workspaces.separator_color,
            );
        }

        draw_rounded_rect(
            &cr,
            &wksp.rect,
            config.workspaces.border_radius,
            config.workspaces.border_width,
            wk_border_color,
            wk_bg_color,
        );

        if wk_show_titles && wksp.tag_name.is_some() && wksp.tag_rect.width() > 0.0 {
            paint_text(
                &cr,
                wksp.tag_name.as_deref().unwrap(),
                &wksp.tag_rect,
                wk_fg_color,
                wk_font_size,
                1.0,
                &config.workspaces.font_family.as_family(),
                false,
                true,
                font_system,
                swash_cache,
                glyph_cache,
            );
        }

        for win in &wksp.windows {
            if win.app_id.is_none() {
                continue;
            };

            let (win_bg_color, win_border_color) = if win.is_focused {
                (
                    config.windows.active_bg_color,
                    config.windows.active_border_color,
                )
            } else {
                (config.windows.bg_color, config.windows.border_color)
            };

            if win.separator_rect.width() > 0.0 {
                draw_rounded_rect(
                    &cr,
                    &win.separator_rect,
                    config.windows.separator_radius,
                    0.0,
                    config.windows.separator_color,
                    config.windows.separator_color,
                );
            }

            // Window Button paint
            draw_rounded_rect(
                &cr,
                &win.rect,
                config.windows.border_radius as f64,
                config.windows.border_width as f64,
                win_border_color,
                win_bg_color,
            );

            // 2. DESENHO DO ÍCONE (Tratamento Híbrido SVG/PNG)

            let icon_opacity = if win.is_focused {
                config.windows.active_icon_opacity
            } else {
                config.windows.icon_opacity
            };

            let base_color = if win.is_focused {
                config.windows.active_fg_color
            } else {
                config.windows.fg_color
            };

            let text_color = cosmic_text::Color::rgba(
                base_color.r(),
                base_color.g(),
                base_color.b(),
                (icon_opacity * 255.0) as u8,
            );

            if has_icon && win.icon_rect.width() > 0.0 {
                if win.has_resolved_icon {
                    if let Some(icon_surface) = icon_surface_cache.get(&win.icon_key) {
                        cr.save().unwrap();
                        if icon_surface.type_() == SurfaceType::Image {
                            cr.set_source_surface(
                                icon_surface,
                                win.icon_rect.x(),
                                win.icon_rect.y(),
                            )
                            .unwrap();
                            cr.paint_with_alpha(icon_opacity).unwrap();
                        } else {
                            cairo_set_color(&cr, text_color);
                            cr.mask_surface(icon_surface, win.icon_rect.x(), win.icon_rect.y())
                                .unwrap();
                        }
                        cr.restore().unwrap();
                    }
                } else {
                    let char_icon: String = if !win.fallback_char.is_empty() {
                        win.fallback_char.to_uppercase().clone()
                    } else {
                        win.title
                            .as_deref()
                            .and_then(|t| t.chars().next())
                            .unwrap_or('?')
                            .to_uppercase()
                            .to_string()
                    };

                    paint_text(
                        &cr,
                        &char_icon,
                        &win.icon_rect,
                        base_color,
                        icon_size as i32,
                        1.0,
                        &config.windows.font_family.as_family(),
                        false,
                        true,
                        font_system,
                        swash_cache,
                        glyph_cache,
                    );
                }
            }

            let has_title = match &config.windows.show_titles {
                ShowTitles::Always | ShowTitles::Only => true,
                ShowTitles::Focused if win.is_focused => true,
                _ => false,
            };

            // skip if not showing titles
            if !has_title {
                continue;
            }

            let title = win.title.as_deref().unwrap_or("");

            if title.is_empty() || win_title_width <= 0.0 {
                continue;
            }

            dbg!(is_vertical);
            paint_text(
                &cr,
                title,
                &win.title_rect,
                base_color,
                win_font_size,
                win_line_height,
                &config.windows.font_family.as_family(),
                win_wrap_titles,
                is_vertical,
                font_system,
                swash_cache,
                glyph_cache,
            );
        }
    }
}
