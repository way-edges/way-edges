# Workspace

The workspace can be changed by either left click, or by only vertical scroll event though mouse wheel or whatever input device that sends `discrete != 0`  
Which means scroll with 2 fingers on touchpad can not trigger anything in this widget. (This should be a behavior defined by your compositor for example niri, using 3 fingers swipe operation to change the workspace)

```jsonc
{
  // ... other basic configs omitted here for brevity
  "widget": {
    "type": "workspace",
    "thickness": 20,
    "length": "25%",
    "active-increase": 0.5, // increase the size of the active workspace hint
    "workspace-animation-curve": "ease-expo",
    "active-color": "#00000000",
    "default-color": "#00000000",
    "focus-color": "#00000000",
    "hover-color": "#00000000",
    "gap": 5,
    "invert-direction": false,
    "output-name": "eDP-1", // not specified, it will use the output that this widget is on
    "pop-duration": 1000, // ms
    "workspace-transition-duration": 300, // ms
    "focused-only": false, // only show animation on the currently focused monitor
    "border-radius": 5,
    "border-width": null,
    // "preset": "hyprland",
    // "preset": "niri",
    "preset": {
      "type": "niri",
      "filter-empty": true,
    },
  },
},
```

| Name                          | Description                                                                |
| ----------------------------- | -------------------------------------------------------------------------- |
| type                          | const `workspace`                                                          |
| thickness                     | can be relative(`xx%`) or a int number                                     |
| length                        | can be relative(`xx%`) or a int number                                     |
| active-increase               | increase the size of the active workspace hint                             |
| active-color                  | active monitor                                                             |
| default-color                 | color                                                                      |
| focus-color                   | color                                                                      |
| hover-color                   | color                                                                      |
| gap                           | gap between each workspace                                                 |
| invert-direction              | invert the direction of the workspace                                      |
| output-name                   | not specified, it will use the output that this widget is on               |
| pop-duration                  | ms                                                                         |
| workspace-transition-duration | ms                                                                         |
| focused-only                  | only show workspaces on focused monitor: `true` or `false`                 |
| workspace-animation-curve     | animation curve                                                            |
| border-radius                 | border radius of the workspace widget                                      |
| border-width                  | border width of the workspace widget, leave `null` will use `thickness/10` |
| preset                        | `hyprland` or `niri` or `niri` with config                                 |

`focused-only`: On multi-monitor setups, when set to `true`, widgets will only animate on the currently focused monitor. When set to `false`, widgets animate on all monitors. This helps prevent unwanted animations on non-focused monitors when switching workspaces. **Available for both niri and Hyprland.**

## Preset: niri

```jsonc
"preset": {
  "type": "niri",
  "filter-empty": true,
},
// or
"preset": "niri",
```

## Preset: hyprland

```jsonc
"preset": "hyprland",
```

## Multi-Monitor Configuration Examples

### Example: Focused-only animations

```jsonc
{
  "preset": "hyprland",
  "focused-only": true, // Only animate on currently focused monitor
}
```

### Example: Niri with focused-only animations

```jsonc
{
  "preset": {
    "type": "niri",
    "filter-empty": true,
  },
  "focused-only": true, // Only animate on currently focused monitor
}
```
