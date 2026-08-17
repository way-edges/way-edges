use cosmic_text::{Color, FamilyOwned};
use knus::{Decode, DecodeScalar};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};
use util::color::parse_color;

use crate::def::shared::{
    color_translate, deserialize_family_owned, dt_family_owned, parse_family_owned, schema_color,
    schema_family_owned, NumMargins,
};

use super::wrapbox::Align;

#[derive(Debug, Default, Copy, Eq, Clone, DecodeScalar, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ShowTitles {
    #[default]
    Always,
    Focused,
    Never,
    Only,
}

fn icon_fallback_deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // 1. Instrui o parser a tentar ler o valor atual como uma String opcional
    let opt_string = Option::<String>::deserialize(deserializer)?;
    match opt_string {
        Some(valor) => {
            let valor_trim = valor.trim();

            if valor_trim.ends_with(".png") || valor_trim.ends_with(".svg") {
                return Ok(Some(valor_trim.to_string()));
            }

            let tamanho = valor_trim.chars().count();
            if tamanho > 0 && tamanho <= 4 {
                return Ok(Some(valor_trim.to_string()));
            }
            Err(serde::de::Error::custom(format!(
                "Invalid icon-fallback: '{}'. Should be only one character, symbol, emoji or a path to an image file ending with .svg or .png.",
                valor_trim
            )))
        }
        None => Ok(None),
    }
}
#[derive(Debug, Clone, Decode, PartialEq, Deserialize, JsonSchema)]
pub struct Windows {
    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    #[serde(default = "dt_family_owned")]
    #[serde(deserialize_with = "deserialize_family_owned")]
    #[schemars(schema_with = "schema_family_owned")]
    pub font_family: FamilyOwned,

    #[knus(child, default = dt_wi_font_size(), unwrap(argument))]
    #[serde(default = "dt_wi_font_size")]
    pub font_size: i32,

    #[knus(child, default = dt_wi_line_height(), unwrap(argument))]
    #[serde(default = "dt_wi_line_height")]
    pub line_height: f64,

    #[knus(child, default = dt_wi_show_titles(), unwrap(argument))]
    #[serde(default = "dt_wi_show_titles")]
    pub show_titles: ShowTitles,

    #[knus(child, default = dt_wi_wrap_titles(), unwrap(argument))]
    #[serde(default = "dt_wi_wrap_titles")]
    pub wrap_titles: bool,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub icon_theme: Option<String>,

    #[knus(child, default = dt_wi_icon_size(), unwrap(argument))]
    #[serde(default = "dt_wi_icon_size")]
    pub icon_size: f64,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    #[serde(deserialize_with = "icon_fallback_deserialize")]
    pub icon_fallback: Option<String>,

    #[knus(child, default = dt_wi_icon_opacity(), unwrap(argument))]
    #[serde(default = "dt_wi_icon_opacity")]
    pub icon_opacity: f64,

    #[knus(child, default = dt_wi_active_icon_opacity(), unwrap(argument))]
    #[serde(default = "dt_wi_active_icon_opacity")]
    pub active_icon_opacity: f64,

    #[knus(child, default = dt_wi_title_width(), unwrap(argument))]
    #[serde(default = "dt_wi_title_width")]
    pub title_width: f64,

    #[knus(child, default = dt_wi_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wi_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub fg_color: Color,

    #[knus(child, default = dt_wi_active_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wi_active_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_fg_color: Color,

    #[knus(child, default = dt_wi_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wi_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub bg_color: Color,

    #[knus(child, default = dt_wi_active_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wi_active_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_bg_color: Color,

    #[knus(child, default = dt_wi_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wi_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,

    #[knus(child, default = dt_wi_active_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wi_active_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_border_color: Color,

    #[knus(child, default = dt_wi_border_width(), unwrap(argument))]
    #[serde(default = "dt_wi_border_width")]
    pub border_width: f64,

    #[knus(child, default = dt_wi_separator_width(), unwrap(argument))]
    #[serde(default = "dt_wi_separator_width")]
    pub separator_width: f64,

    #[knus(child, default = dt_wi_separator_margin(), unwrap(argument))]
    #[serde(default = "dt_wi_separator_margin")]
    pub separator_margin: f64,

    #[knus(child, default = dt_wi_separator_radius(), unwrap(argument))]
    #[serde(default = "dt_wi_separator_radius")]
    pub separator_radius: f64,

    #[knus(child, default = dt_wi_separator_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wi_separator_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub separator_color: Color,

    #[knus(child, default = dt_wi_border_radius(), unwrap(argument))]
    #[serde(default = "dt_wi_border_radius")]
    pub border_radius: f64,

    #[knus(child, default = dt_wi_gap(), unwrap(argument))]
    #[serde(default = "dt_wi_gap")]
    pub gap: f64,

    #[knus(child, default = dt_wi_margin())]
    #[serde(default = "dt_wi_margin")]
    pub margins: NumMargins,
}

impl Default for Windows {
    fn default() -> Self {
        Self {
            font_family: dt_family_owned(),
            font_size: dt_wi_font_size(),
            line_height: dt_wi_line_height(),
            title_width: dt_wi_title_width(),
            show_titles: dt_wi_show_titles(),
            wrap_titles: dt_wi_wrap_titles(),
            icon_theme: None,
            icon_size: dt_wi_icon_size(),
            icon_fallback: None,
            icon_opacity: dt_wi_icon_opacity(),
            active_icon_opacity: dt_wi_active_icon_opacity(),
            fg_color: dt_wi_fg_color(),
            active_fg_color: dt_wi_active_fg_color(),
            bg_color: dt_wi_bg_color(),
            active_bg_color: dt_wi_active_bg_color(),
            border_color: dt_wi_border_color(),
            active_border_color: dt_wi_active_border_color(),
            border_width: dt_wi_border_width(),
            border_radius: dt_wi_border_radius(),
            separator_color: dt_wi_separator_color(),
            separator_width: dt_wi_separator_width(),
            separator_margin: dt_wi_separator_margin(),
            separator_radius: dt_wi_separator_radius(),
            gap: dt_wi_gap(),
            margins: dt_wi_margin(),
        }
    }
}

#[derive(Debug, Clone, Decode, PartialEq, Deserialize, JsonSchema)]
pub struct Workspaces {
    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    #[serde(default = "dt_family_owned")]
    #[serde(deserialize_with = "deserialize_family_owned")]
    #[schemars(schema_with = "schema_family_owned")]
    pub font_family: FamilyOwned,

    #[knus(child, default = dt_wk_font_size(), unwrap(argument))]
    #[serde(default = "dt_wk_font_size")]
    pub font_size: i32,

    #[knus(child, default = dt_wk_show_titles(), unwrap(argument))]
    #[serde(default = "dt_wk_show_titles")]
    pub show_titles: bool,

    #[knus(child, default = dt_wk_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wk_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub fg_color: Color,

    #[knus(child, default = dt_wk_active_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wk_active_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_fg_color: Color,

    #[knus(child, default = dt_wk_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wk_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub bg_color: Color,

    #[knus(child, default = dt_wk_active_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wk_active_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_bg_color: Color,

    #[knus(child, default = dt_wk_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wk_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,

    #[knus(child, default = dt_wk_active_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wk_active_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_border_color: Color,

    #[knus(child, default = dt_wk_border_width(), unwrap(argument))]
    #[serde(default = "dt_wk_border_width")]
    pub border_width: f64,

    #[knus(child, default = dt_wk_border_radius(), unwrap(argument))]
    #[serde(default = "dt_wk_border_radius")]
    pub border_radius: f64,

    #[knus(child, default = dt_wk_separator_width(), unwrap(argument))]
    #[serde(default = "dt_wk_separator_width")]
    pub separator_width: f64,

    #[knus(child, default = dt_wk_separator_margin(), unwrap(argument))]
    #[serde(default = "dt_wk_separator_margin")]
    pub separator_margin: f64,

    #[knus(child, default = dt_wk_separator_radius(), unwrap(argument))]
    #[serde(default = "dt_wk_separator_radius")]
    pub separator_radius: f64,

    #[knus(child, default = dt_wk_separator_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wk_separator_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub separator_color: Color,

    #[knus(child, default = dt_wk_gap(), unwrap(argument))]
    #[serde(default = "dt_wk_gap")]
    pub gap: f64,

    #[knus(child, default = dt_wk_margin())]
    #[serde(default = "dt_wk_margin")]
    pub margins: NumMargins,
}

impl Default for Workspaces {
    fn default() -> Self {
        Self {
            font_family: dt_family_owned(),
            font_size: dt_wk_font_size(),
            show_titles: dt_wk_show_titles(),
            fg_color: dt_wk_fg_color(),
            active_fg_color: dt_wk_active_fg_color(),
            bg_color: dt_wk_bg_color(),
            active_bg_color: dt_wk_active_bg_color(),
            border_color: dt_wk_border_color(),
            active_border_color: dt_wk_active_border_color(),
            border_width: dt_wk_border_width(),
            border_radius: dt_wk_border_radius(),
            separator_color: dt_wk_separator_color(),
            separator_width: dt_wk_separator_width(),
            separator_margin: dt_wk_separator_margin(),
            separator_radius: dt_wk_separator_radius(),
            gap: dt_wk_gap(),
            margins: dt_margin(),
        }
    }
}

#[derive(Debug, Decode, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(deny_unknown_fields)]
pub struct DockConfig {
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub grid_align: Align,

    #[knus(child, default = dt_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub color: Color,

    #[knus(child, default = dt_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,

    #[knus(child, default = dt_wk_border_width(), unwrap(argument))]
    #[serde(default = "dt_border_width")]
    pub border_width: f64,

    #[knus(child, default = dt_wk_border_radius(), unwrap(argument))]
    #[serde(default = "dt_border_radius")]
    pub border_radius: f64,

    #[knus(child, default = dt_margin())]
    #[serde(default = "dt_margin")]
    pub margins: NumMargins,

    #[knus(child, default = dt_workspaces())]
    #[serde(default = "dt_workspaces")]
    pub workspaces: Workspaces,

    #[knus(child, default = dt_windows())]
    #[serde(default = "dt_windows")]
    pub windows: Windows,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            grid_align: Align::default(),
            color: dt_color(),
            border_color: dt_border_color(),
            border_width: dt_border_width(),
            border_radius: dt_border_radius(),
            margins: dt_margin(),
            workspaces: Workspaces::default(),
            windows: Windows::default(),
        }
    }
}

// Geração em massa das funções de fallback exigidas pelo parser
macro_rules! def_fallback {
    ($func_name:ident, $ret_type:ty, $value:expr) => {
        pub fn $func_name() -> $ret_type {
            $value
        }
    };
}
def_fallback!(dt_color, Color, Color::rgba(0, 0, 0, 255));
def_fallback!(dt_border_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_border_width, f64, 0.0);
def_fallback!(dt_border_radius, f64, 10.0);
def_fallback!(
    dt_margin,
    NumMargins,
    NumMargins {
        left: 3,
        right: 3,
        top: 3,
        bottom: 3
    }
);

def_fallback!(dt_workspaces, Workspaces, Workspaces::default());
def_fallback!(dt_wk_font_size, i32, 26);
def_fallback!(dt_wk_show_titles, bool, false);
def_fallback!(dt_wk_fg_color, Color, Color::rgba(255, 255, 255, 80));
def_fallback!(
    dt_wk_active_fg_color,
    Color,
    Color::rgba(255, 255, 255, 160)
);
def_fallback!(dt_wk_bg_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_wk_active_bg_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_wk_border_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_wk_active_border_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_wk_border_width, f64, 0.0);
def_fallback!(dt_wk_border_radius, f64, 10.0);
def_fallback!(dt_wk_separator_width, f64, 1.0);
def_fallback!(dt_wk_separator_margin, f64, 6.0);
def_fallback!(dt_wk_separator_radius, f64, 0.0);
def_fallback!(dt_wk_separator_color, Color, Color::rgba(255, 255, 255, 50));
def_fallback!(dt_wk_gap, f64, 20.0);
def_fallback!(
    dt_wk_margin,
    NumMargins,
    NumMargins {
        left: 3,
        right: 3,
        top: 3,
        bottom: 3
    }
);

// window_button WindowButton
def_fallback!(dt_windows, Windows, Windows::default());
def_fallback!(dt_wi_font_size, i32, 16);
def_fallback!(dt_wi_line_height, f64, 1.4);
def_fallback!(dt_wi_title_width, f64, 60.0);
def_fallback!(dt_wi_show_titles, ShowTitles, ShowTitles::Focused);
def_fallback!(dt_wi_wrap_titles, bool, false);
def_fallback!(dt_wi_icon_size, f64, 32.0);
def_fallback!(dt_wi_icon_opacity, f64, 0.5);
def_fallback!(dt_wi_active_icon_opacity, f64, 1.0);
def_fallback!(dt_wi_fg_color, Color, Color::rgba(255, 255, 255, 80));
def_fallback!(
    dt_wi_active_fg_color,
    Color,
    Color::rgba(255, 255, 255, 160)
);
def_fallback!(dt_wi_bg_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_wi_active_bg_color, Color, Color::rgba(255, 255, 255, 17));
def_fallback!(dt_wi_border_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(
    dt_wi_active_border_color,
    Color,
    Color::rgba(255, 255, 255, 30)
);
def_fallback!(dt_wi_border_width, f64, 1.0);
def_fallback!(dt_wi_border_radius, f64, 5.0);
def_fallback!(dt_wi_separator_width, f64, 0.0);
def_fallback!(dt_wi_separator_margin, f64, 6.0);
def_fallback!(dt_wi_separator_radius, f64, 0.0);
def_fallback!(dt_wi_separator_color, Color, Color::rgba(255, 255, 255, 50));
def_fallback!(dt_wi_gap, f64, 5.0);
def_fallback!(
    dt_wi_margin,
    NumMargins,
    NumMargins {
        left: 5,
        right: 5,
        top: 5,
        bottom: 5
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_dock_configs() {
        let kdl = r##"
dock {
    edge "bottom"
    grid-align "center-center"
    color "#111111"
    border-color "#111111"
    border-width 2
    border-radius 15
    margins {
        left 4
        right 4
        top 4
        bottom 4
    }
    workspaces{
        font-family "mono"
        font-size 12
        show-titles true
        fg-color "#ffffff"
        active-fg-color "#0000ff"
        bg-color "#000000"
        active-bg-color "#00ff00"
        border-color "#333333"
        active-border-color "#222222"
        border-width 2
        border-radius 15
        separator-width 2.0
        separator-margin 5.0
        separator-radius 2.0
        separator-color "#333333"
        gap 8
        margins {
            left 4
            right 4
            top 4
            bottom 4
        }
    }
    window-button {
        font-family "mono"
        font-size 12
        line-height 1.3
        title-width 48
        show-titles "always"
        wrap-titles true
        icon-theme "Adwaita"
        icon-size 24
        icon-fallback "?"
        icon-opacity 0.5
        active-icon-opacity 1
        fg-color "#ffffff"
        active-fg-color "#0000ff"
        bg-color "#000000"
        active-bg-color "#00ff00"
        border-color "#000000"
        active-border-color "#000000"
        border-width 2
        border-radius 15
        separator-width 2.0
        separator-margin 5.0
        separator-radius 2.0
        separator-color "#333333"
        gap 4
        margins {
            left 2
            right 2
            top 2
            bottom 2
        }
    }
}
"##;
        let parsed: Vec<crate::def::WidgetConf> = knus::parse("test", kdl).unwrap();
        if let crate::def::WidgetConf::Dock(dock) = &parsed[0] {
            let dock_config = &dock.widget;
            assert!(matches!(dock_config.grid_align, Align::CenterCenter));
            assert_eq!(dock_config.color, parse_color("#111111").unwrap());
            assert_eq!(dock_config.border_color, parse_color("#111111").unwrap());
            assert_eq!(dock_config.border_width, 2.0);
            assert_eq!(dock_config.border_radius, 15.0);
            assert_eq!(
                dock_config.margins,
                NumMargins {
                    left: 4,
                    right: 4,
                    top: 4,
                    bottom: 4,
                }
            );

            //workspaces
            assert_eq!(dock_config.workspaces.font_size, 12);
            assert_eq!(dock_config.workspaces.show_titles, true);
            assert_eq!(
                dock_config.workspaces.fg_color,
                parse_color("#ffffff").unwrap()
            );
            assert_eq!(
                dock_config.workspaces.active_fg_color,
                parse_color("#0000ff").unwrap()
            );
            assert_eq!(
                dock_config.workspaces.bg_color,
                parse_color("#000000").unwrap()
            );
            assert_eq!(
                dock_config.workspaces.active_bg_color,
                parse_color("#00ff00").unwrap()
            );
            assert_eq!(
                dock_config.workspaces.border_color,
                parse_color("#333333").unwrap()
            );
            assert_eq!(
                dock_config.workspaces.active_border_color,
                parse_color("#222222").unwrap()
            );
            assert_eq!(dock_config.workspaces.border_width, 2.0);
            assert_eq!(dock_config.workspaces.border_radius, 15.0);
            assert_eq!(dock_config.workspaces.separator_width, 2.0);
            assert_eq!(dock_config.workspaces.separator_margin, 5.0);
            assert_eq!(dock_config.workspaces.separator_radius, 2.0);
            assert_eq!(
                dock_config.workspaces.separator_color,
                parse_color("#333333").unwrap()
            );
            assert_eq!(dock_config.workspaces.gap, 8.0);
            assert_eq!(
                dock_config.workspaces.margins,
                NumMargins {
                    left: 4,
                    right: 4,
                    top: 4,
                    bottom: 4,
                }
            );

            //window_button
            assert_eq!(dock_config.windows.font_size, 12);
            assert_eq!(dock_config.windows.title_width, 48.0);
            assert!(matches!(
                dock_config.windows.show_titles,
                ShowTitles::Always
            ));
            assert_eq!(dock_config.windows.wrap_titles, true);
            assert_eq!(dock_config.windows.icon_theme.as_ref().unwrap(), "Adwaita");
            assert_eq!(dock_config.windows.icon_size, 24.0);
            assert_eq!(dock_config.windows.line_height, 1.3);
            assert_eq!(dock_config.windows.icon_opacity, 0.5);
            assert_eq!(dock_config.windows.active_icon_opacity, 1.0);
            assert_eq!(dock_config.windows.icon_fallback.as_ref().unwrap(), "?");
            assert_eq!(
                dock_config.windows.fg_color,
                parse_color("#ffffff").unwrap()
            );
            assert_eq!(
                dock_config.windows.active_fg_color,
                parse_color("#0000ff").unwrap()
            );
            assert_eq!(
                dock_config.windows.bg_color,
                parse_color("#000000").unwrap()
            );
            assert_eq!(
                dock_config.windows.active_bg_color,
                parse_color("#00ff00").unwrap()
            );
            assert_eq!(
                dock_config.windows.border_color,
                parse_color("#000000").unwrap()
            );
            assert_eq!(
                dock_config.windows.active_border_color,
                parse_color("#000000").unwrap()
            );
            assert_eq!(dock_config.windows.border_width, 2.0);
            assert_eq!(dock_config.windows.border_radius, 15.0);
            assert_eq!(dock_config.windows.separator_width, 2.0);
            assert_eq!(dock_config.windows.separator_margin, 5.0);
            assert_eq!(dock_config.windows.separator_radius, 2.0);
            assert_eq!(
                dock_config.windows.separator_color,
                parse_color("#333333").unwrap()
            );
            assert_eq!(dock_config.windows.gap, 4.0);
            assert_eq!(
                dock_config.windows.margins,
                NumMargins {
                    left: 2,
                    right: 2,
                    top: 2,
                    bottom: 2,
                }
            );
        } else {
            panic!("Expected Dock widget");
        }
    }
}
