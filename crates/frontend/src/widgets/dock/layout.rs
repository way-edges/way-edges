use std::collections::HashMap;

use backend::dock::icons::IconKey;
use backend::dock::DockData;
use cairo::{ImageSurface, Rectangle};
use config::def::shared::NumMargins;
use config::def::widgets::dock::{DockConfig, ShowTitles};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};

#[derive(Debug, Clone, Copy)]
pub struct MarginsF64 {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl From<&NumMargins> for MarginsF64 {
    fn from(m: &NumMargins) -> Self {
        Self {
            left: m.left as f64,
            right: m.right as f64,
            top: m.top as f64,
            bottom: m.bottom as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DockWindow {
    pub rect: Rectangle,
    pub icon_rect: Rectangle,
    pub title_rect: Rectangle,
    pub separator_rect: Rectangle,
    pub id: f64,
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub is_focused: bool,
    pub has_resolved_icon: bool,
    pub icon_key: IconKey,
    pub fallback_char: String,
}

impl DockWindow {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.rect.x()
            && px <= (self.rect.x() + self.rect.width())
            && py >= self.rect.y()
            && py <= (self.rect.y() + self.rect.height())
    }
}

#[derive(Debug, Clone)]
pub struct DockWorkspace {
    pub rect: Rectangle,
    pub tag_rect: Rectangle,
    pub tag_name: Option<String>,
    pub is_focused: bool,
    pub separator_rect: Rectangle,
    pub windows: Vec<DockWindow>,
}

impl DockWorkspace {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.rect.x()
            && px <= (self.rect.x() + self.rect.width())
            && py >= self.rect.y()
            && py <= (self.rect.y() + self.rect.height())
    }
}

#[derive(Debug, Clone)]
pub struct DockLayout {
    pub rect: Rectangle,
    pub workspaces: Vec<DockWorkspace>,
    pub is_vertical: bool,
}

impl DockLayout {
    pub fn new() -> Self {
        DockLayout {
            rect: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            workspaces: Vec::new(),
            is_vertical: false,
        }
    }

    pub fn calculate(
        dock_data: &DockData,
        config: &DockConfig,
        icon_surface_cache: &HashMap<IconKey, ImageSurface>,
        font_system: &mut FontSystem,
        is_vertical: bool,
    ) -> Self {
        let dock_border_width = config.border_width;
        let dock_margins: MarginsF64 = (&config.margins).into();

        let wk_border_width = config.workspaces.border_width;
        let wk_gap = config.workspaces.gap;
        let wk_margins: MarginsF64 = (&config.workspaces.margins).into();
        let wk_show_titles = config.workspaces.show_titles;
        let wk_font_size = config.workspaces.font_size as f32;
        let wk_y = dock_border_width + dock_margins.top;

        let win_y = wk_y + wk_border_width + wk_margins.top;
        let win_border_width = config.windows.border_width;
        let win_margins: MarginsF64 = (&config.windows.margins).into();
        let win_gap = config.windows.gap;

        let icon_size = config.windows.icon_size;
        let icon_theme = &config.windows.icon_theme;
        let icon_fallback = &config.windows.icon_fallback;
        let win_show_titles = config.windows.show_titles;
        let is_icon_enabled = win_show_titles != ShowTitles::Only && icon_size > 0.0;

        let win_title_width = config.windows.title_width;

        let win_height = icon_size + win_border_width * 2.0 + win_margins.top + win_margins.bottom;
        let wk_height = win_height + wk_border_width * 2.0 + wk_margins.top + wk_margins.bottom;
        let dock_height =
            wk_height + dock_border_width * 2.0 + dock_margins.top + dock_margins.bottom;

        let mut current_main_axis = dock_border_width + dock_margins.left;

        let workspaces: Vec<DockWorkspace> = dock_data
            .windows
            .chunk_by(|a, b| a.workspace_id == b.workspace_id)
            .enumerate()
            .map(|(idx, wins)| {
                let wk_id = wins[0].workspace_id.unwrap_or(0) as i32;
                let wk_x = current_main_axis;

                current_main_axis += wk_border_width + wk_margins.left;

                let name = dock_data
                    .workspaces
                    .get(&(wk_id as u64))
                    .map(|w| w.name.clone().or_else(|| Some(w.idx.to_string())))
                    .unwrap_or_default();

                let tag_rect = if name.is_some() && wk_show_titles {
                    let (rect_width, rect_height) = measure_text(
                        font_system,
                        name.as_deref().unwrap(),
                        wk_font_size,
                        config.workspaces.font_family.as_family(),
                    );

                    // Workspace Tag won't be transposed, will stay uprigth no matter if the dock
                    // is horizontal or vetical, so won't use the fn verticalize_rec. But it's x and
                    // y will still swap places in the vertical layout

                    let rect = if is_vertical {
                        Rectangle::new(
                            wk_y.round(),
                            current_main_axis.round(),
                            wk_height.round(),
                            rect_height.round(),
                        )
                    } else {
                        Rectangle::new(
                            current_main_axis.round(),
                            wk_y.round(),
                            rect_width.round(),
                            wk_height.round(),
                        )
                    };

                    current_main_axis +=
                        wk_margins.left as f64 + if is_vertical { rect_height } else { rect_width };

                    rect
                } else {
                    Rectangle::new(0.0, 0.0, 0.0, 0.0)
                };

                let mut wk_focused = false;

                let windows: Vec<DockWindow> = wins
                    .iter()
                    .enumerate()
                    .map(|(win_idx, win)| {
                        wk_focused = wk_focused || win.is_focused;

                        let has_title = match win_show_titles {
                            ShowTitles::Always | ShowTitles::Only => true,
                            ShowTitles::Focused if win.is_focused => true,
                            _ => false,
                        };
                        let win_x = current_main_axis;

                        let app_id = win.app_id.clone().unwrap_or_default();

                        let icon_key = IconKey {
                            app_id: app_id.clone(),
                            theme: icon_theme.clone(),
                            size: icon_size as u32,
                            fallback: icon_fallback.clone(),
                        };

                        current_main_axis += win_margins.left + win_border_width;

                        let (icon_rect, has_resolved_icon, fallback_char) = if is_icon_enabled {
                            let (icon_width, icon_height, has_resolved_icon) =
                                if let Some(surface) = icon_surface_cache.get(&icon_key) {
                                    (surface.width() as f64, surface.height() as f64, true)
                                } else {
                                    (icon_size, icon_size, false)
                                };

                            let icon_rec = verticalize_rect(
                                current_main_axis,
                                win_y + win_margins.top + win_border_width,
                                icon_width,
                                icon_height,
                                is_vertical,
                            );

                            (icon_rec, has_resolved_icon, String::new())
                        } else {
                            let fallback_char = if let Some(fb) =
                                icon_fallback.as_deref().filter(|s| s.chars().count() == 1)
                            {
                                fb.to_string()
                            } else if let Some(title) =
                                win.title.as_deref().filter(|s| !s.is_empty())
                            {
                                title.chars().next().unwrap().to_uppercase().to_string()
                            } else {
                                String::new()
                            };

                            let (width, height) = measure_text(
                                font_system,
                                &fallback_char,
                                wk_font_size,
                                config.workspaces.font_family.as_family(),
                            );

                            let icon_rec = verticalize_rect(
                                current_main_axis,
                                win_y + win_margins.top + win_border_width,
                                width,
                                height,
                                is_vertical,
                            );

                            (icon_rec, false, fallback_char)
                        };

                        current_main_axis += icon_rect.width();

                        let title_rect = if has_title {
                            if is_icon_enabled && icon_rect.width() > 0.0 {
                                current_main_axis += win_margins.left;
                            }
                            let rec = verticalize_rect(
                                current_main_axis,
                                win_y + win_margins.top,
                                win_title_width,
                                icon_size,
                                is_vertical,
                            );

                            current_main_axis += win_title_width;

                            rec
                        } else {
                            Rectangle::new(0.0, 0.0, 0.0, 0.0)
                        };

                        current_main_axis += win_margins.right + win_border_width;
                        let win_width = current_main_axis - win_x;

                        let win_rec =
                            verticalize_rect(win_x, win_y, win_width, win_height, is_vertical);

                        let win_sep_height = win_height - 2.0 * config.windows.separator_margin;
                        let win_separator_rect = if config.windows.separator_width > 0.0
                            && config.windows.separator_width < win_gap as f64
                            && win_sep_height > 0.0
                            && win_idx > 0
                        {
                            let sep_x = win_x - (config.windows.separator_width + win_gap) / 2.0;
                            let sep_y = win_y + config.windows.separator_margin;

                            verticalize_rect(
                                sep_x,
                                sep_y,
                                config.windows.separator_width,
                                win_sep_height,
                                is_vertical,
                            )
                        } else {
                            Rectangle::new(0.0, 0.0, 0.0, 0.0)
                        };

                        current_main_axis += win_gap;

                        DockWindow {
                            rect: win_rec,
                            icon_rect,
                            title_rect,
                            separator_rect: win_separator_rect,
                            id: win.id as f64,
                            app_id: win.app_id.clone(),
                            title: win.title.clone(),
                            is_focused: win.is_focused,
                            has_resolved_icon,
                            icon_key,
                            fallback_char,
                        }
                    })
                    .collect();

                current_main_axis -= win_gap; // last item has no gap
                current_main_axis += wk_margins.right + wk_border_width;

                let wk_width = current_main_axis - wk_x;

                let wk_rect = verticalize_rect(wk_x, wk_y, wk_width, wk_height, is_vertical);

                let wk_sep_height = wk_height - 2.0 * config.workspaces.separator_margin;
                let wk_separator_rect = if config.workspaces.separator_width > 0.0
                    && config.workspaces.separator_width < wk_gap as f64
                    && wk_sep_height > 0.0
                    && idx > 0
                {
                    let sep_x = wk_x - (config.workspaces.separator_width + wk_gap) / 2.0;
                    let sep_y = wk_y + config.workspaces.separator_margin;

                    verticalize_rect(
                        sep_x,
                        sep_y,
                        config.workspaces.separator_width,
                        wk_sep_height,
                        is_vertical,
                    )
                } else {
                    Rectangle::new(0.0, 0.0, 0.0, 0.0)
                };

                current_main_axis += wk_gap;

                DockWorkspace {
                    rect: wk_rect,
                    tag_rect,
                    tag_name: name,
                    is_focused: wk_focused,
                    separator_rect: wk_separator_rect,
                    windows,
                }
            })
            .collect();

        current_main_axis -= wk_gap; // last workspace has no gap
        current_main_axis += dock_margins.right + dock_border_width;

        let dock_rect = verticalize_rect(0.0, 0.0, current_main_axis, dock_height, is_vertical);

        DockLayout {
            rect: dock_rect,
            workspaces,
            is_vertical,
        }
    }

    pub fn find_clicked_item(&self, x: f64, y: f64) -> Option<&DockWindow> {
        self.workspaces
            .iter()
            .find(|ws| ws.contains(x, y))
            .and_then(|ws| ws.windows.iter().find(|win| win.contains(x, y)))
    }
}

fn verticalize_rect(x: f64, y: f64, w: f64, h: f64, is_vertical: bool) -> Rectangle {
    let rx = x.round();
    let ry = y.round();
    let rw = w.round();
    let rh = h.round();
    if is_vertical {
        Rectangle::new(ry, rx, rh, rw)
    } else {
        Rectangle::new(rx, ry, rw, rh)
    }
}

pub fn measure_text(
    font_system: &mut FontSystem,
    text: &str,
    font_size: f32,
    font_family: Family,
) -> (f64, f64) {
    let metrics = Metrics::new(font_size, font_size);
    let mut buffer = Buffer::new(font_system, metrics);

    let attrs = Attrs::new().family(font_family);
    buffer.set_text(font_system, text, &attrs, Shaping::Advanced);
    buffer.shape_until_scroll(font_system, false);

    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let height = buffer.layout_runs().count() as f32 * font_size;

    (width as f64, height as f64)
}
