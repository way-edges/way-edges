# Workspace

```jsonc
{
  // ... other basic configs omitted here for brevity
  "widget": {
    "type": "workspace",
    "thickness": 20,
    "length": "25%",
    "active_increase": 0.5, // increase the size of the active workspace hint
    "animation_curve": "ease-expo",
    "active_color": "#00000000",
    "default_color": "#00000000",
    "focus_color": "#00000000",
    "hover_color": "#00000000",
    "gap": 5,
    "invert_direction": false,
    "output_name": "eDP-1", // not specified, it will use the output that this widget is on
    "pop_duration": 1000, // ms
    "workspace_transition_duration": 300, // ms
    // "preset": "hyprland",
    // "preset": "niri",
    "preset": {
      "type": "niri",
      "filter_empty": true,
    },
  },
},
```

| Name                          | Description                                                  |
| ----------------------------- | ------------------------------------------------------------ |
| type                          | const `workspace`                                            |
| thickness                     | can be relative(`xx%`) or a int number                       |
| length                        | can be relative(`xx%`) or a int number                       |
| active_increase               | increase the size of the active workspace hint               |
| active_color                  | active monitor                                               |
| default_color                 | color                                                        |
| focus_color                   | color                                                        |
| hover_color                   | color                                                        |
| gap                           | gap between each workspace                                   |
| invert_direction              | invert the direction of the workspace                        |
| output_name                   | not specified, it will use the output that this widget is on |
| pop_duration                  | ms                                                           |
| workspace_transition_duration | ms                                                           |
| animation_curve               | animation curve                                              |
| preset                        | `hyprland` or `niri` or `niri` with config                   |

## Preset: niri

```jsonc
"preset": {
  "type": "niri",
  "filter_empty": true,
},
// or
"preset": "niri",
```

| Name         | Description            |
| ------------ | ---------------------- |
| type         | const `niri`           |
| filter_empty | ignore empty workspace |

## Preset: hyprland

```jsonc
"preset": "hyprland",
```
