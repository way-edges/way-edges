use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

pub fn find_icon(icon_theme_path: &str, icon_name: &str) -> Option<PathBuf> {
    if let Some(path) = try_cached(icon_theme_path, icon_name) {
        return Some(path);
    }

    walkdir::WalkDir::new(icon_theme_path)
        .max_depth(1)
        .contents_first(true)
        .into_iter()
        .find_map(|r| {
            let r = match r {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Error walking dir: {e}");
                    return None;
                }
            };

            let (Some(before), Some(after)) = rsplit_file_at_dot(r.file_name()) else {
                return None;
            };

            match after.as_encoded_bytes() {
                b"png" | b"svg" => {
                    if before.as_encoded_bytes() == icon_name.as_bytes() {
                        Some(r.into_path())
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .inspect(|f| {
            insert_to_cache(icon_theme_path, icon_name, f.clone());
        })
}

fn rsplit_file_at_dot(file: &OsStr) -> (Option<&OsStr>, Option<&OsStr>) {
    if file.as_encoded_bytes() == b".." {
        return (Some(file), None);
    }

    // The unsafety here stems from converting between &OsStr and &[u8]
    // and back. This is safe to do because (1) we only look at ASCII
    // contents of the encoding and (2) new &OsStr values are produced
    // only from ASCII-bounded slices of existing &OsStr values.
    let mut iter = file.as_encoded_bytes().rsplitn(2, |b| *b == b'.');
    let after = iter.next();
    let before = iter.next();
    if before == Some(b"") {
        (Some(file), None)
    } else {
        unsafe {
            (
                before.map(|s| OsStr::from_encoded_bytes_unchecked(s)),
                after.map(|s| OsStr::from_encoded_bytes_unchecked(s)),
            )
        }
    }
}

type NameCache = BTreeMap<String, PathBuf>;
type ThemePathCache = BTreeMap<String, NameCache>;

#[derive(Default)]
struct Cache(Mutex<ThemePathCache>);
static CACHE: LazyLock<Cache> = LazyLock::new(Cache::default);

fn try_cached(icon_theme_path: &str, icon_name: &str) -> Option<PathBuf> {
    CACHE
        .0
        .lock()
        .unwrap()
        .get(icon_theme_path)?
        .get(icon_name)
        .cloned()
}

fn insert_to_cache(icon_theme_path: &str, icon_name: &str, path: PathBuf) {
    let mut theme_map = CACHE.0.lock().unwrap();

    match theme_map.get_mut(icon_theme_path) {
        Some(icon_map) => {
            icon_map.insert(icon_name.to_string(), path);
        }
        None => {
            let mut icon_map = BTreeMap::new();
            icon_map.insert(icon_name.to_string(), path);
            theme_map.insert(icon_theme_path.to_string(), icon_map);
        }
    }
}
