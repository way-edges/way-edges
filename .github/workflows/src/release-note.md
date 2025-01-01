# 💻 Way-Edges

**Breaking**:

## Feature

- **Breaking**: `transition_duration` & `frame_rate` & `extra_trigger_size` is moved into common widget config, you should remove them in widget specific config.
- **Breaking**: `event_map` for `Btn` & `Slider` are changed from tuple to map, you should change the config.
- **Breaking**: widget type of `Slider` & `WrapBox` & `HyprWorkspace` are change to `slider` `wrap-box` `hypr-workspace` respectively, you should do the same in your config.
- **Breaking**: `preset` for `Slider` & `Ring` & `Text` should always be an `object`, not `string`, you should change that in your code.
- **Breaking**: `OutlookMargins` is no longer a tuple but a map.
- `frame_rate` now default to your monitor refresh rate, feel free to remove it in your config.
- `extra_trigger_size` is default to 1 pixel, feel free to remove it in your config.
- `preview_size` to make some widget reveal some of their content constantly.
- `border_width` & `border_color` config for `Btn`.
- `invert_diraction` config for `HyprWorkspace`.
- `redraw_only_on_internal_update` for `Slider` to save resources, default false, but recommend to turn it on for `Speaker` & `Microphone`.
- Custom preset for `Slider`, `on_change` added Template usage.
- Each ring preset has `update_interval` config.
- Template functionality.

## Fix

- Backlight can only watch for one device.

## Improvement

- Less memory usage. Reduce image buffer cache, and only redraw when data or animation update to save resources.
- Render performance improvement. Remove condition match in each redraw call, create and match function for each condition, save them as function pointer.
- Smoother frame rate. Remove `tokio-timerfd`, integrate timerfd frame management in glib, no channel or lock overhead.
- Remove channel usage for rendering, reduce overhead and keep frame up to date.
- Use tokio for hyprland listener, reduce additional thread.
- Remove channel usage for PulseAudio, improve performance for `Speaker` and `Microphone`.
- Replace `get_sys_info` with `sysinfo` and `starship-battery`, less code to maintain and performance improvement.
- Less code for configuration, thus better.
- Improve animation logic.
- Unify mouse event handling logic.
- Unify edge window related logic, reduce code complexity.
- Improve `Slider` & `WrapBox` render logic.
- `Slider` style change.
- `Btn` style improvement.
