use std::{path::PathBuf, sync::LazyLock};

static DEFAULT_ICON_THEME: LazyLock<Option<String>> = LazyLock::new(linicon_theme::get_icon_theme);

pub fn fallback_icon(size: i32, theme: Option<&str>) -> Option<PathBuf> {
    let mut builder = freedesktop_icons::lookup("image-missing")
        .with_size(size as u16)
        .with_size_scheme(freedesktop_icons::SizeScheme::LargerClosest)
        .with_cache();
    if let Some(t) = theme.or(DEFAULT_ICON_THEME.as_deref()) {
        builder = builder.with_theme(t);
    }
    builder.find()
}
pub fn find_icon(name: &str, size: i32, theme: Option<&str>) -> Option<PathBuf> {
    let mut builder = freedesktop_icons::lookup(name)
        .with_size(size as u16)
        .with_size_scheme(freedesktop_icons::SizeScheme::LargerClosest);
    if let Some(t) = theme.or(DEFAULT_ICON_THEME.as_deref()) {
        builder = builder.with_theme(t);
    }
    builder.find()
}
