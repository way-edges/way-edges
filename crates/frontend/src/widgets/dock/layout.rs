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
    pub id: u64,
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub is_focused: bool,
    pub hover_opacity: f64,
    pub has_resolved_icon: bool,
    pub icon_key: IconKey,
    pub fallback_char: String,
}

impl DockWindow {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.rect.x()
            && px < (self.rect.x() + self.rect.width())
            && py >= self.rect.y()
            && py < (self.rect.y() + self.rect.height())
    }
}

#[derive(Debug, Clone)]
pub struct DockWorkspace {
    pub id: Option<u64>,
    pub rect: Rectangle,
    pub tag_rect: Rectangle,
    pub tag_text_rect: Rectangle,
    pub tag_name: Option<String>,
    pub status_rect: Rectangle,
    pub rail_rect: Rectangle,
    pub is_focused: bool,
    pub separator_rect: Rectangle,
    pub windows: Vec<DockWindow>,
}

impl DockWorkspace {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.rect.x()
            && px < (self.rect.x() + self.rect.width())
            && py >= self.rect.y()
            && py < (self.rect.y() + self.rect.height())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusOutline {
    pub rect: Rectangle,
    pub marker: Option<(f64, f64)>,
    pub opacity: f64,
}

#[derive(Debug, Clone)]
pub struct DockLayout {
    pub rect: Rectangle,
    pub workspaces: Vec<DockWorkspace>,
    pub is_vertical: bool,
    pub selection: Option<FocusOutline>,
    pub hovered_window: Option<u64>,
}

impl DockLayout {
    pub fn new() -> Self {
        DockLayout {
            rect: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            workspaces: Vec::new(),
            is_vertical: false,
            selection: None,
            hovered_window: None,
        }
    }

    pub fn calculate(
        dock_data: &DockData,
        config: &DockConfig,
        icon_surface_cache: &HashMap<IconKey, ImageSurface>,
        font_system: &mut FontSystem,
        is_vertical: bool,
    ) -> Self {
        let dock_border = config.border_width;
        let dm: MarginsF64 = (&config.margins).into();
        let wb = config.workspaces.border_width;
        let wm: MarginsF64 = (&config.workspaces.margins).into();
        let win_border = config.windows.border_width;
        let win_m: MarginsF64 = (&config.windows.margins).into();
        let icon_size = config.windows.icon_size;
        let icons_enabled = config.windows.show_titles != ShowTitles::Only && icon_size > 0.0;
        let has_title = |window: &backend::dock::DockWindowData| {
            config.windows.title_width > 0.0
                && match config.windows.show_titles {
                    ShowTitles::Always | ShowTitles::Only => true,
                    ShowTitles::Focused => window.is_focused,
                    ShowTitles::Never => false,
                }
        };
        let icon_key = |window: &backend::dock::DockWindowData| IconKey {
            app_id: window.app_id.clone().unwrap_or_default(),
            theme: config.windows.icon_theme.clone(),
            size: icon_size as u32,
            fallback: config.windows.icon_fallback.clone(),
        };
        let icon_dimensions = |window: &backend::dock::DockWindowData| {
            if !icons_enabled {
                return (0.0, 0.0);
            }
            icon_surface_cache
                .get(&icon_key(window))
                .map(|surface| (surface.width() as f64, surface.height() as f64))
                .unwrap_or((icon_size, icon_size))
        };
        let title_gap = win_m.left.max(8.0);
        let title_widths: HashMap<_, _> = dock_data
            .windows
            .iter()
            .filter(|window| has_title(window))
            .map(|window| {
                let width = measure_text(
                    font_system,
                    window.title.as_deref().unwrap_or(""),
                    config.windows.font_size as f32,
                    config.windows.font_family.as_family(),
                )
                .0;
                (window.id, width.min(config.windows.title_width))
            })
            .collect();
        let window_width = |window: &backend::dock::DockWindowData| {
            let icon_width = icon_dimensions(window).0;
            let title_width = title_widths.get(&window.id).copied().unwrap_or(0.0);
            (win_border * 2.0
                + win_m.left
                + win_m.right
                + icon_width
                + title_width
                + if icon_width > 0.0 && title_width > 0.0 {
                    title_gap
                } else {
                    0.0
                })
            .ceil()
        };
        let text_height = if dock_data.windows.iter().any(has_title) {
            config.windows.font_size as f64 * config.windows.line_height
        } else {
            0.0
        };
        let row_height =
            (icon_size.max(text_height) + win_border * 2.0 + win_m.top + win_m.bottom).ceil();
        let header_font = config.workspaces.font_size as f32;
        let status_font = (config.workspaces.font_size as f64 * 0.75)
            .round()
            .max(10.0) as f32;
        let status_width = measure_text(
            font_system,
            "FOCUS",
            status_font,
            config.workspaces.font_family.as_family(),
        )
        .0;
        let groups: Vec<_> = dock_data
            .windows
            .chunk_by(|a, b| a.workspace_id == b.workspace_id)
            .map(|windows| {
                let workspace = windows[0]
                    .workspace_id
                    .and_then(|id| dock_data.workspaces.get(&id));
                let label = workspace
                    .map(workspace_label)
                    .filter(|_| config.workspaces.show_titles);
                let focused = windows.iter().any(|win| win.is_focused)
                    || workspace.is_some_and(|workspace| workspace.is_focused);
                let label_width = label
                    .as_deref()
                    .map(|label| {
                        measure_text(
                            font_system,
                            label,
                            header_font,
                            config.workspaces.font_family.as_family(),
                        )
                        .0
                    })
                    .unwrap_or(0.0);
                let header_width = label_width
                    + if focused && label.is_some() {
                        status_width + 16.0
                    } else {
                        0.0
                    };
                (windows, label, focused, header_width)
            })
            .collect();
        let header_height = if groups.iter().any(|(_, label, _, _)| label.is_some()) {
            header_font as f64 + 18.0 + config.workspaces.rail_width.max(0.0)
        } else {
            0.0
        };
        let inner_cross = if is_vertical {
            groups
                .iter()
                .map(|(windows, _, _, header_width)| {
                    windows
                        .iter()
                        .map(window_width)
                        .fold(*header_width, f64::max)
                })
                .fold(row_height, f64::max)
        } else {
            header_height + row_height
        };
        let cross_size = inner_cross
            + wb * 2.0
            + if is_vertical {
                wm.left + wm.right
            } else {
                wm.top + wm.bottom
            };
        let mut main = dock_border + if is_vertical { dm.top } else { dm.left };
        let mut workspaces = Vec::new();
        for (index, (windows, label, focused, header_width)) in groups.into_iter().enumerate() {
            let window_extent = if is_vertical {
                row_height * windows.len() as f64
            } else {
                windows.iter().map(window_width).sum()
            };
            let window_extent = window_extent + config.windows.gap * (windows.len() - 1) as f64;
            let inner_width = if is_vertical {
                inner_cross
            } else {
                window_extent.max(header_width)
            };
            let group_width = if is_vertical {
                cross_size
            } else {
                inner_width + wb * 2.0 + wm.left + wm.right
            };
            let group_height = if is_vertical {
                header_height + window_extent + wb * 2.0 + wm.top + wm.bottom
            } else {
                cross_size
            };
            let group_x = if is_vertical {
                dock_border + dm.left
            } else {
                main
            };
            let group_y = if is_vertical {
                main
            } else {
                dock_border + dm.top
            };
            let rect = Rectangle::new(group_x, group_y, group_width.ceil(), group_height.ceil());
            let inner_x = group_x + wb + wm.left;
            let inner_y = group_y + wb + wm.top;
            let tag_rect = if label.is_some() {
                Rectangle::new(
                    inner_x,
                    inner_y,
                    header_width - if focused { status_width + 16.0 } else { 0.0 },
                    header_font as f64,
                )
            } else {
                Rectangle::new(0.0, 0.0, 0.0, 0.0)
            };
            let status_rect = if label.is_some() && focused {
                Rectangle::new(
                    inner_x + inner_width - status_width,
                    inner_y + (header_font as f64 - status_font as f64) / 2.0,
                    status_width,
                    status_font as f64,
                )
            } else {
                Rectangle::new(0.0, 0.0, 0.0, 0.0)
            };
            let rail_rect = if label.is_some() {
                Rectangle::new(
                    inner_x,
                    inner_y + header_font as f64 + 7.0,
                    inner_width,
                    config.workspaces.rail_width.max(0.0),
                )
            } else {
                Rectangle::new(0.0, 0.0, 0.0, 0.0)
            };
            let mut cursor = 0.0;
            let mut laid_out_windows = Vec::new();
            for (win_index, window) in windows.iter().enumerate() {
                let x = inner_x + if is_vertical { 0.0 } else { cursor };
                let y = inner_y + header_height + if is_vertical { cursor } else { 0.0 };
                let width = if is_vertical {
                    inner_width
                } else {
                    window_width(window)
                };
                let leading = if win_index > 0 {
                    config.windows.gap.max(0.0) / 2.0
                } else {
                    0.0
                };
                let trailing = if win_index + 1 < windows.len() {
                    config.windows.gap.max(0.0) / 2.0
                } else {
                    0.0
                };
                // Adjacent cells meet at the midpoint of the space between their contents.
                let win_rect = if is_vertical {
                    Rectangle::new(x, y - leading, width, row_height + leading + trailing)
                } else {
                    Rectangle::new(x - leading, y, width + leading + trailing, row_height)
                };
                let (icon_width, icon_height) = icon_dimensions(window);
                let icon_rect = Rectangle::new(
                    x + win_border + win_m.left,
                    y + (row_height - icon_height) / 2.0,
                    icon_width,
                    icon_height,
                );
                let title_rect = if has_title(window) {
                    let title_x =
                        icon_rect.x() + icon_width + if icon_width > 0.0 { title_gap } else { 0.0 };
                    Rectangle::new(
                        title_x,
                        y + win_border + win_m.top,
                        title_widths[&window.id]
                            .min(x + width - win_border - win_m.right - title_x),
                        row_height - win_border * 2.0 - win_m.top - win_m.bottom,
                    )
                } else {
                    Rectangle::new(0.0, 0.0, 0.0, 0.0)
                };
                let separator = config.windows.separator_width;
                let separator_margin = config.windows.separator_margin;
                let separator_rect =
                    if win_index > 0 && separator > 0.0 && separator < config.windows.gap {
                        if is_vertical {
                            Rectangle::new(
                                x + separator_margin,
                                y - (separator + config.windows.gap) / 2.0,
                                (width - separator_margin * 2.0).max(0.0),
                                separator,
                            )
                        } else {
                            Rectangle::new(
                                x - (separator + config.windows.gap) / 2.0,
                                y + separator_margin,
                                separator,
                                (row_height - separator_margin * 2.0).max(0.0),
                            )
                        }
                    } else {
                        Rectangle::new(0.0, 0.0, 0.0, 0.0)
                    };
                let key = icon_key(window);
                let has_resolved_icon = icon_surface_cache.contains_key(&key);
                laid_out_windows.push(DockWindow {
                    rect: win_rect,
                    icon_rect,
                    title_rect,
                    separator_rect,
                    id: window.id,
                    app_id: window.app_id.clone(),
                    title: window.title.clone(),
                    is_focused: window.is_focused,
                    hover_opacity: 0.0,
                    has_resolved_icon,
                    icon_key: key,
                    fallback_char: config
                        .windows
                        .icon_fallback
                        .as_deref()
                        .filter(|s| s.chars().count() == 1)
                        .unwrap_or("")
                        .to_string(),
                });
                cursor += if is_vertical { row_height } else { width };
                cursor += config.windows.gap;
            }
            let separator = config.workspaces.separator_width;
            let separator_margin = config.workspaces.separator_margin;
            let separator_rect =
                if index > 0 && separator > 0.0 && separator < config.workspaces.gap {
                    if is_vertical {
                        Rectangle::new(
                            group_x + separator_margin,
                            group_y - (separator + config.workspaces.gap) / 2.0,
                            (group_width - separator_margin * 2.0).max(0.0),
                            separator,
                        )
                    } else {
                        Rectangle::new(
                            group_x - (separator + config.workspaces.gap) / 2.0,
                            group_y + separator_margin,
                            separator,
                            (group_height - separator_margin * 2.0).max(0.0),
                        )
                    }
                } else {
                    Rectangle::new(0.0, 0.0, 0.0, 0.0)
                };
            workspaces.push(DockWorkspace {
                id: windows[0].workspace_id,
                rect,
                tag_rect,
                tag_text_rect: tag_rect,
                tag_name: label,
                status_rect,
                rail_rect,
                is_focused: focused,
                separator_rect,
                windows: laid_out_windows,
            });
            main += if is_vertical {
                group_height
            } else {
                group_width
            };
            main += config.workspaces.gap;
        }
        if !workspaces.is_empty() {
            main -= config.workspaces.gap;
        }
        main += dock_border + if is_vertical { dm.bottom } else { dm.right };
        let cross = cross_size
            + dock_border * 2.0
            + if is_vertical {
                dm.left + dm.right
            } else {
                dm.top + dm.bottom
            };
        let selection = workspaces.iter().find_map(|workspace| {
            workspace
                .windows
                .iter()
                .find(|window| window.is_focused)
                .map(|window| {
                    let marker = if is_vertical {
                        Some((
                            (window.rect.x() - 6.0).max(workspace.rect.x() + 4.0),
                            window.rect.y() + window.rect.height() / 2.0,
                        ))
                    } else if workspace.rail_rect.width() > 0.0
                        && workspace.rail_rect.height() > 0.0
                    {
                        Some((
                            if window.icon_rect.width() > 0.0 {
                                window.icon_rect.x() + window.icon_rect.width() / 2.0
                            } else {
                                window.rect.x() + window.rect.width() / 2.0
                            },
                            workspace.rail_rect.y() + workspace.rail_rect.height() / 2.0,
                        ))
                    } else {
                        None
                    };
                    FocusOutline {
                        rect: window.rect,
                        marker,
                        opacity: 1.0,
                    }
                })
        });
        Self {
            rect: Rectangle::new(
                0.0,
                0.0,
                if is_vertical {
                    cross.ceil()
                } else {
                    main.ceil()
                },
                if is_vertical {
                    main.ceil()
                } else {
                    cross.ceil()
                },
            ),
            workspaces,
            is_vertical,
            selection,
            hovered_window: None,
        }
    }

    pub fn same_geometry(&self, other: &Self) -> bool {
        self.rect == other.rect
            && self.selection == other.selection
            && self.is_vertical == other.is_vertical
            && self.workspaces.len() == other.workspaces.len()
            && self.workspaces.iter().zip(&other.workspaces).all(|(a, b)| {
                a.id == b.id
                    && a.rect == b.rect
                    && a.tag_rect == b.tag_rect
                    && a.tag_text_rect == b.tag_text_rect
                    && a.status_rect == b.status_rect
                    && a.rail_rect == b.rail_rect
                    && a.separator_rect == b.separator_rect
                    && a.windows.len() == b.windows.len()
                    && a.windows.iter().zip(&b.windows).all(|(a, b)| {
                        a.id == b.id
                            && a.rect == b.rect
                            && a.icon_rect == b.icon_rect
                            && a.title_rect == b.title_rect
                            && a.separator_rect == b.separator_rect
                    })
            })
    }

    pub fn interpolate(from: &Self, to: &Self, progress: f64) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        if progress >= 1.0 {
            return to.clone();
        }
        let mut layout = to.clone();
        layout.rect = interpolate_rect(from.rect, to.rect, progress);
        for workspace in &mut layout.workspaces {
            if let Some(previous) = from.workspaces.iter().find(|old| old.id == workspace.id) {
                workspace.rect = interpolate_rect(previous.rect, workspace.rect, progress);
                workspace.tag_rect =
                    interpolate_rect(previous.tag_rect, workspace.tag_rect, progress);
                workspace.tag_text_rect =
                    interpolate_rect(previous.tag_text_rect, workspace.tag_text_rect, progress);
                workspace.rail_rect =
                    interpolate_rect(previous.rail_rect, workspace.rail_rect, progress);
                if workspace.status_rect.width() > 0.0
                    && workspace.rail_rect.width()
                        < workspace.tag_text_rect.width() + workspace.status_rect.width() + 16.0
                {
                    workspace.status_rect = Rectangle::new(0.0, 0.0, 0.0, 0.0);
                }
                if workspace.status_rect.width() > 0.0 {
                    workspace.status_rect.set_x(
                        workspace.rail_rect.x() + workspace.rail_rect.width()
                            - workspace.status_rect.width(),
                    );
                    workspace.status_rect.set_y(
                        workspace.tag_rect.y()
                            + (workspace.tag_rect.height() - workspace.status_rect.height()) / 2.0,
                    );
                }
                workspace.separator_rect =
                    interpolate_rect(previous.separator_rect, workspace.separator_rect, progress);
            }
            for window in &mut workspace.windows {
                if let Some(previous) = from
                    .workspaces
                    .iter()
                    .flat_map(|ws| &ws.windows)
                    .find(|old| old.id == window.id)
                {
                    let target_rect = window.rect;
                    let target_icon = window.icon_rect;
                    let target_title = window.title_rect;
                    window.rect = interpolate_rect(previous.rect, window.rect, progress);
                    window.icon_rect =
                        interpolate_rect(previous.icon_rect, window.icon_rect, progress);
                    window.title_rect =
                        if previous.title_rect.width() <= 0.0 && target_title.width() > 0.0 {
                            Rectangle::new(
                                window.icon_rect.x() + window.icon_rect.width() + target_title.x()
                                    - target_icon.x()
                                    - target_icon.width(),
                                window.rect.y() + target_title.y() - target_rect.y(),
                                target_title.width() * progress,
                                target_title.height(),
                            )
                        } else {
                            interpolate_rect(previous.title_rect, target_title, progress)
                        };
                    window.separator_rect =
                        interpolate_rect(previous.separator_rect, window.separator_rect, progress);
                }
            }
        }
        layout.selection = match (from.selection, to.selection) {
            (Some(old), Some(new)) => Some(FocusOutline {
                rect: interpolate_rect(old.rect, new.rect, progress),
                marker: match (old.marker, new.marker) {
                    (Some(a), Some(b)) => Some((
                        interpolate_value(a.0, b.0, progress),
                        interpolate_value(a.1, b.1, progress),
                    )),
                    (_, target) => target,
                },
                opacity: interpolate_value(old.opacity, new.opacity, progress),
            }),
            (Some(mut old), None) => {
                old.opacity *= 1.0 - progress;
                Some(old)
            }
            (None, Some(mut new)) => {
                new.opacity *= progress;
                Some(new)
            }
            (None, None) => None,
        };
        layout
    }

    pub fn update_hover(&mut self, position: Option<(f64, f64)>) -> bool {
        let hovered =
            position.and_then(|(x, y)| self.find_clicked_item(x, y).map(|window| window.id));
        let changed = self.hovered_window != hovered;
        self.hovered_window = hovered;
        for window in self
            .workspaces
            .iter_mut()
            .flat_map(|workspace| &mut workspace.windows)
        {
            window.hover_opacity = if hovered == Some(window.id) { 1.0 } else { 0.0 };
        }
        changed
    }

    pub fn find_clicked_item(&self, x: f64, y: f64) -> Option<&DockWindow> {
        self.workspaces
            .iter()
            .find(|ws| ws.contains(x, y))
            .and_then(|ws| ws.windows.iter().find(|win| win.contains(x, y)))
    }
}

fn interpolate_value(from: f64, to: f64, progress: f64) -> f64 {
    from + (to - from) * progress
}

fn interpolate_rect(from: Rectangle, to: Rectangle, progress: f64) -> Rectangle {
    Rectangle::new(
        interpolate_value(from.x(), to.x(), progress),
        interpolate_value(from.y(), to.y(), progress),
        interpolate_value(from.width(), to.width(), progress),
        interpolate_value(from.height(), to.height(), progress),
    )
}

fn workspace_label(workspace: &backend::dock::DockWorkspaceData) -> String {
    match workspace.name.as_deref().filter(|name| !name.is_empty()) {
        Some(name) => format!(
            "WS {:02} / {}",
            workspace.idx,
            name.split_whitespace().collect::<Vec<_>>().join(" ")
        ),
        None => format!("WS {:02}", workspace.idx),
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

    ((width as f64).ceil(), (height as f64).ceil())
}

#[cfg(test)]
mod tests {
    use super::*;
    use backend::dock::{DockWindowData, DockWorkspaceData};

    #[test]
    fn empty_dock_keeps_its_margins_in_both_orientations() {
        let config = DockConfig::default();
        let mut fonts = FontSystem::new();
        let icons = HashMap::new();
        for vertical in [false, true] {
            let layout =
                DockLayout::calculate(&DockData::default(), &config, &icons, &mut fonts, vertical);
            let main_axis = if vertical {
                layout.rect.height()
            } else {
                layout.rect.width()
            };
            assert_eq!(
                main_axis,
                (config.margins.left + config.margins.right) as f64
            );
            assert!(layout.rect.width() > 0.0 && layout.rect.height() > 0.0);
            assert!(layout.find_clicked_item(0.0, 0.0).is_none());
        }
    }

    #[test]
    fn hit_testing_preserves_full_window_and_workspace_ids() {
        let mut config = DockConfig::default();
        config.workspaces.show_titles = true;
        config.windows.icon_fallback = Some("?".into());
        let workspace_id = u64::MAX - 1;
        let data = DockData {
            windows: vec![DockWindowData {
                id: u64::MAX,
                workspace_id: Some(workspace_id),
                app_id: None,
                title: Some("A window".into()),
                is_focused: true,
                is_floating: false,
                is_urgent: false,
            }],
            workspaces: [(
                workspace_id,
                DockWorkspaceData {
                    id: workspace_id,
                    idx: 1,
                    name: Some("Workspace".into()),
                    output: Some("A".into()),
                    is_active: true,
                    is_focused: true,
                },
            )]
            .into(),
        };
        let mut fonts = FontSystem::new();
        for vertical in [false, true] {
            let layout =
                DockLayout::calculate(&data, &config, &HashMap::new(), &mut fonts, vertical);
            assert_eq!(
                layout.workspaces[0].tag_name.as_deref(),
                Some("WS 01 / Workspace")
            );
            let workspace = &layout.workspaces[0];
            let (text_width, text_height) = measure_text(
                &mut fonts,
                workspace.tag_name.as_deref().unwrap(),
                config.workspaces.font_size as f32,
                config.workspaces.font_family.as_family(),
            );
            assert!(workspace.tag_text_rect.width() >= text_width);
            assert!(workspace.tag_text_rect.height() >= text_height);
            assert!(workspace.tag_rect.x() >= workspace.rect.x());
            assert!(
                workspace.tag_rect.x() + workspace.tag_rect.width()
                    <= workspace.rect.x() + workspace.rect.width()
            );
            let win = &workspace.windows[0];
            assert!(workspace.tag_rect.y() + workspace.tag_rect.height() < win.rect.y());
            let clicked = layout
                .find_clicked_item(
                    win.rect.x() + win.rect.width() / 2.0,
                    win.rect.y() + win.rect.height() / 2.0,
                )
                .unwrap();
            assert_eq!(clicked.id, u64::MAX);
            assert_eq!(clicked.fallback_char, "?");

            let surface = ImageSurface::create(
                cairo::Format::ARgb32,
                layout.rect.width() as i32,
                layout.rect.height() as i32,
            )
            .unwrap();
            crate::widgets::dock::draw::paint(
                &surface,
                &layout,
                &config,
                &HashMap::new(),
                &mut fonts,
                &mut cosmic_text::SwashCache::new(),
                &mut HashMap::new(),
            );
            surface.status().unwrap();
        }
    }

    #[test]
    fn focus_switches_between_distinct_workspace_groups() {
        let mut config = DockConfig::default();
        config.workspaces.show_titles = true;
        let mut data = DockData {
            workspaces: [(1, "code"), (2, "browser")]
                .into_iter()
                .map(|(id, name)| {
                    (
                        id,
                        DockWorkspaceData {
                            id,
                            idx: id,
                            name: Some(name.into()),
                            output: Some("A".into()),
                            is_active: false,
                            is_focused: false,
                        },
                    )
                })
                .collect(),
            windows: [(10, 1), (11, 1), (20, 2)]
                .into_iter()
                .map(|(id, workspace)| DockWindowData {
                    id,
                    workspace_id: Some(workspace),
                    app_id: None,
                    title: Some("Window".into()),
                    is_focused: false,
                    is_floating: false,
                    is_urgent: false,
                })
                .collect(),
        };
        let mut fonts = FontSystem::new();
        for vertical in [false, true] {
            for (focus, workspace_id) in [(11, 1), (20, 2)] {
                for window in &mut data.windows {
                    window.is_focused = window.id == focus;
                }
                for workspace in data.workspaces.values_mut() {
                    workspace.is_focused = workspace.id == workspace_id;
                }
                let layout =
                    DockLayout::calculate(&data, &config, &HashMap::new(), &mut fonts, vertical);
                assert_eq!(
                    layout.workspaces.iter().filter(|ws| ws.is_focused).count(),
                    1
                );
                let selected_group = layout.workspaces.iter().find(|ws| ws.is_focused).unwrap();
                assert!(selected_group.status_rect.width() > 0.0);
                assert!(
                    selected_group.tag_text_rect.x() + selected_group.tag_text_rect.width()
                        < selected_group.status_rect.x()
                );
                for workspace in &layout.workspaces {
                    for window in &workspace.windows {
                        assert!(window.rect.y() > workspace.rail_rect.y());
                        assert!(
                            window.rect.x() + window.rect.width()
                                <= workspace.rect.x() + workspace.rect.width()
                        );
                        assert!(
                            window.rect.y() + window.rect.height()
                                <= workspace.rect.y() + workspace.rect.height()
                        );
                        assert_eq!(
                            layout
                                .find_clicked_item(
                                    window.rect.x() + window.rect.width() / 2.0,
                                    window.rect.y() + window.rect.height() / 2.0
                                )
                                .unwrap()
                                .id,
                            window.id
                        );
                    }
                }
                let selected = selected_group
                    .windows
                    .iter()
                    .find(|win| win.is_focused)
                    .unwrap();
                assert_eq!(selected.id, focus);
                assert!(selected.title_rect.width() > 0.0);
                assert!(layout
                    .workspaces
                    .iter()
                    .flat_map(|ws| &ws.windows)
                    .filter(|win| !win.is_focused)
                    .all(|win| win.title_rect.width() == 0.0));
                let first = &layout.workspaces[0].rect;
                let second = &layout.workspaces[1].rect;
                if vertical {
                    assert!(first.y() + first.height() < second.y());
                } else {
                    assert!(first.x() + first.width() < second.x());
                }
            }
        }
    }

    #[test]
    fn selection_resizes_and_retargets_from_the_displayed_frame() {
        let mut config = DockConfig::default();
        config.workspaces.show_titles = true;
        let mut data = DockData {
            workspaces: [(
                1,
                DockWorkspaceData {
                    id: 1,
                    idx: 1,
                    name: Some("code".into()),
                    output: Some("A".into()),
                    is_active: true,
                    is_focused: true,
                },
            )]
            .into(),
            windows: [(10, "A long terminal window title"), (11, "Music")]
                .into_iter()
                .map(|(id, title)| DockWindowData {
                    id,
                    workspace_id: Some(1),
                    app_id: None,
                    title: Some(title.into()),
                    is_focused: id == 10,
                    is_floating: false,
                    is_urgent: false,
                })
                .collect(),
        };
        let mut other_workspace = data.workspaces[&1].clone();
        other_workspace.id = 2;
        other_workspace.idx = 2;
        other_workspace.name = Some("music".into());
        other_workspace.is_focused = false;
        data.workspaces.insert(2, other_workspace);
        data.windows[1].workspace_id = Some(2);
        let mut fonts = FontSystem::new();
        for vertical in [false, true] {
            data.windows[0].is_focused = true;
            data.windows[1].is_focused = false;
            data.workspaces.get_mut(&1).unwrap().is_focused = true;
            data.workspaces.get_mut(&2).unwrap().is_focused = false;
            let from = DockLayout::calculate(&data, &config, &HashMap::new(), &mut fonts, vertical);
            data.windows[0].is_focused = false;
            data.windows[1].is_focused = true;
            data.workspaces.get_mut(&1).unwrap().is_focused = false;
            data.workspaces.get_mut(&2).unwrap().is_focused = true;
            let to = DockLayout::calculate(&data, &config, &HashMap::new(), &mut fonts, vertical);
            if !vertical {
                assert!(from.selection.unwrap().rect.width() > to.selection.unwrap().rect.width());
            }
            let halfway = DockLayout::interpolate(&from, &to, 0.5);
            let rect = halfway.selection.unwrap().rect;
            let start = from.selection.unwrap().rect;
            let end = to.selection.unwrap().rect;
            assert_eq!(rect.x(), (start.x() + end.x()) / 2.0);
            assert_eq!(rect.y(), (start.y() + end.y()) / 2.0);
            assert_eq!(rect.width(), (start.width() + end.width()) / 2.0);
            for workspace in &halfway.workspaces {
                if workspace.status_rect.width() > 0.0 {
                    assert!(
                        workspace.tag_text_rect.x() + workspace.tag_text_rect.width()
                            < workspace.status_rect.x()
                    );
                }
            }
            for window in halfway
                .workspaces
                .iter()
                .flat_map(|workspace| &workspace.windows)
            {
                if window.is_focused {
                    assert!(window.title_rect.x() >= window.rect.x());
                    assert!(window.title_rect.y() >= window.rect.y());
                    assert!(
                        window.title_rect.x() + window.title_rect.width()
                            <= window.rect.x() + window.rect.width() + 0.001
                    );
                }
                assert_eq!(
                    halfway
                        .find_clicked_item(
                            window.rect.x() + window.rect.width() / 2.0,
                            window.rect.y() + window.rect.height() / 2.0
                        )
                        .unwrap()
                        .id,
                    window.id
                );
            }
            assert!(DockLayout::interpolate(&from, &to, 1.0).same_geometry(&to));
            let retargeted = DockLayout::interpolate(&halfway, &from, 0.0);
            assert_eq!(retargeted.selection, halfway.selection);
            assert_eq!(retargeted.rect, halfway.rect);
            data.windows
                .iter_mut()
                .for_each(|window| window.is_focused = false);
            let unfocused =
                DockLayout::calculate(&data, &config, &HashMap::new(), &mut fonts, vertical);
            assert_eq!(
                DockLayout::interpolate(&to, &unfocused, 0.5)
                    .selection
                    .unwrap()
                    .opacity,
                0.5
            );
            assert!(DockLayout::interpolate(&to, &unfocused, 1.0)
                .selection
                .is_none());
        }
    }
    #[test]
    fn adjacent_window_cells_have_no_click_dead_zones_and_hover_tracks_the_cell() {
        let mut config = DockConfig::default();
        config.workspaces.show_titles = true;
        let mut data = DockData {
            workspaces: [(
                1,
                DockWorkspaceData {
                    id: 1,
                    idx: 1,
                    name: Some("code".into()),
                    output: Some("A".into()),
                    is_active: true,
                    is_focused: true,
                },
            )]
            .into(),
            windows: (1..=3)
                .map(|id| DockWindowData {
                    id,
                    workspace_id: Some(1),
                    app_id: None,
                    title: Some(format!("Window {id}")),
                    is_focused: id == 1,
                    is_floating: false,
                    is_urgent: false,
                })
                .collect(),
        };
        let mut fonts = FontSystem::new();
        for vertical in [false, true] {
            for gap in [0.0, 14.0] {
                config.windows.gap = gap;
                let mut layout =
                    DockLayout::calculate(&data, &config, &HashMap::new(), &mut fonts, vertical);
                let windows = &layout.workspaces[0].windows;
                for pair in windows.windows(2) {
                    let a = pair[0].rect;
                    let b = pair[1].rect;
                    let (end, start) = if vertical {
                        (a.y() + a.height(), b.y())
                    } else {
                        (a.x() + a.width(), b.x())
                    };
                    assert!((end - start).abs() < 0.0001);
                    for (offset, expected) in
                        [(-0.1, pair[0].id), (0.0, pair[1].id), (0.1, pair[1].id)]
                    {
                        let (x, y) = if vertical {
                            (a.x() + a.width() / 2.0, start + offset)
                        } else {
                            (start + offset, a.y() + a.height() / 2.0)
                        };
                        assert_eq!(layout.find_clicked_item(x, y).unwrap().id, expected);
                    }
                }
                let first = windows[0].rect;
                let last = windows[2].rect;
                let start = if vertical { first.y() } else { first.x() } as i32;
                let end = if vertical {
                    last.y() + last.height()
                } else {
                    last.x() + last.width()
                } as i32;
                for position in start..end {
                    let point = if vertical {
                        (first.x() + first.width() / 2.0, position as f64)
                    } else {
                        (position as f64, first.y() + first.height() / 2.0)
                    };
                    let id = layout.find_clicked_item(point.0, point.1).unwrap().id;
                    layout.update_hover(Some(point));
                    assert_eq!(layout.hovered_window, Some(id));
                }
                assert!(!layout.update_hover(Some((
                    last.x() + last.width() / 2.0,
                    last.y() + last.height() / 2.0
                ))));
                assert!(layout.update_hover(None));
                assert!(layout.hovered_window.is_none());
                data.windows[0].is_focused = false;
                data.windows[2].is_focused = true;
                let target =
                    DockLayout::calculate(&data, &config, &HashMap::new(), &mut fonts, vertical);
                let mut moving = DockLayout::interpolate(&layout, &target, 0.5);
                let point = (
                    first.x() + first.width() / 2.0,
                    first.y() + first.height() / 2.0,
                );
                let expected = moving
                    .find_clicked_item(point.0, point.1)
                    .map(|window| window.id);
                moving.update_hover(Some(point));
                assert_eq!(moving.hovered_window, expected);
                data.windows[0].is_focused = true;
                data.windows[2].is_focused = false;
            }
        }
    }
}
