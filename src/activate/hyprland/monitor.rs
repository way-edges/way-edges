use std::collections::HashMap;

use crate::activate::hyprland::{NAMESPACE_BR, NAMESPACE_TL};
use crate::activate::{get_monitor_index_by_name, set_working_area_size_map_multiple};
use crate::config::GroupConfig;
use gtk::gdk::{Monitor, Rectangle};
use hyprland::data::{LayerClient, Layers};
use hyprland::shared::HyprData;

/// level 2 means `Top` layer
const TOP_LEVEL: &str = "2";

/// Check if window with `namespace` exists,
/// raise error if they exist twice.
/// We can only calculate with the window position on left-top and bottom right,
/// if multiple given, we can not determine.
struct NameSpaceMatch(HashMap<String, bool>, usize);
impl NameSpaceMatch {
    fn new(vs: Vec<String>) -> Self {
        NameSpaceMatch(HashMap::from_iter(vs.into_iter().map(|s| (s, false))), 0)
    }
    fn ok(&mut self, s: &String) -> Result<bool, String> {
        if let Some(b) = self.0.get(s) {
            if *b {
                Err(format!("{s} found twice"))
            } else {
                self.0.insert(s.clone(), true);
                self.1 += 1;
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }
    fn is_finish(&self) -> bool {
        self.1 == self.0.len()
    }
}

/// the calculated size of the available layer area size for each monitor
/// not each layer specific, at lease for hyprland 0.40.0
// pub type MonitorLayerSizeMap = HashMap<Monitor, (i32, i32)>;
// pub type MonitorLayerSizeMap = Vec<Option<(usize, Rectangle)>>;
pub type MonitorLayerSize = (usize, Rectangle);

/// calculate available layer area
pub fn get_monitor_map(
    mut needed_monitors: HashMap<String, ()>,
    // ) -> Result<MonitorLayerSizeMap, String> {
) -> Result<Vec<usize>, String> {
    let mls = Layers::get().map_err(|e| format!("Failed to get layer info: {e}"))?;
    log::debug!("Layer shells from hyprland: {mls:?}");
    let res = mls
        .into_iter()
        .map(|(ms, mut d)| {
            // check if the monitor is the needed one
            log::debug!("Layer shell into monitor: {ms:?}");
            if needed_monitors.remove_entry(&ms).is_none() {
                return Ok(None);
            };
            // just assume that `TOP` layer always exists
            let vc = d.levels.remove(TOP_LEVEL).unwrap();
            let mut lcs: (Option<Box<LayerClient>>, Option<Box<LayerClient>>) = (None, None);
            let mut nsm =
                NameSpaceMatch::new(vec![NAMESPACE_TL.to_string(), NAMESPACE_BR.to_string()]);
            {
                // multiple namespace window, can not determine which to use to calculate size
                // so raise error
                vc.into_iter().try_for_each(|c| -> Result<(), String> {
                    if nsm.ok(&c.namespace)? {
                        match c.namespace.as_str() {
                            NAMESPACE_TL => {
                                lcs.0 = Some(Box::new(c));
                            }
                            NAMESPACE_BR => {
                                lcs.1 = Some(Box::new(c));
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                })?;
            };
            {
                // if 2 positioning window all exist
                if nsm.is_finish() {
                    log::debug!("Layer client for monitor({ms}): {lcs:?}");
                    // top left
                    let tl = lcs.0.unwrap();
                    let start_x = tl.x;
                    let start_y = tl.y;

                    // bottom right
                    let br = lcs.1.unwrap();
                    let end_x = br.x + br.w as i32;
                    let end_y = br.y + br.h as i32;

                    // calculate
                    let w = end_x - start_x;
                    let h = end_y - start_y;
                    let index = match get_monitor_index_by_name(&ms) {
                        Ok(i) => i,
                        Err(e) => {
                            return Err(e);
                        }
                    };

                    Ok(Some((index, Rectangle::new(start_x, start_y, w, h))))
                } else {
                    Ok(None)
                }
            }
        })
        .collect::<Result<Vec<Option<MonitorLayerSize>>, String>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<MonitorLayerSize>>();
    // check if all the needed monitors' size are calculated
    if needed_monitors.is_empty() {
        let indexs = res.iter().map(|(i, _)| *i).collect();
        set_working_area_size_map_multiple(res)?;
        Ok(indexs)
    } else {
        Err(format!(
            "Needed monitors not cleared, remaining: {needed_monitors:?}"
        ))
    }
}

/// get monitors specified in config
pub fn get_need_monitors<'a>(
    cfgs: &GroupConfig,
    monitors: &'a [Monitor],
) -> Result<Vec<&'a Monitor>, String> {
    let mut mm = HashMap::new();
    cfgs.iter().try_for_each(|cfg| -> Result<(), String> {
        let monitor = crate::activate::find_monitor(monitors, &cfg.monitor)?;
        mm.entry(monitor).or_insert(());
        Ok(())
    })?;
    let res = mm.into_keys().collect();
    log::debug!("Needed monitors: {res:?}");
    Ok(res)
}
