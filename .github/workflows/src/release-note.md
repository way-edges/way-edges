# 💻 Way-Edges

## Important

- **(Breaking) Add: workspace added `focus` and `active` state for multi-monitor, configuration changed.**
- **Add: Animation curve - `EaseQuad` `EaseCubic`(default) `EaseExpo` `Linear`**
- **Add: `ensure_load_group` for adding groups on program start, no need for calling `way-edges add` afterwards**
- **Deprecate `daemon` command, launching the program directly with `way-edges`**
- **Add: tray menu `icon_size`; tray `font_family`**
- **Fix: show mouse key debug option not working**

## Less important

- **Refactor: Every default transition duration is changed to 300ms**
- **Add: IPC `reload` command**
- **Add: Battery state for ring preset**
- **Refactor: tray widget(no use related change)**
- **Fix: `hypr` workspace filtering out empty workspaces**
- **Add: Support `empty-workspace-above-first` for niri**
- **Add: Support not filtering empty workspace for niri**

- Bump dep
- Remove freetype dependency
- Remove data cache in system-tray client
- Don't minify schema anymore
- Remove example config, use my dots as example instead
