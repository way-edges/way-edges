# Widget

```jsonc
{
  "name": "widget_base_example",
  "edge": "top",
  "position": "left",
  "layer": "overlay",
  // "monitor": 0,
  // "monitor": "*",
  // "monitor": "eDP-1",
  "monitor": ["eDP-1", "HDMI-A-1"],
  "extra_trigger_size": 1, // or "10%"
  "preview_size": 20, // or "100%"
  "animation_curve": "ease-expo",
  "transition_duration": 300,
  "margins": {
    "top": 0,
    "left": 0,
    "bottom": 0,
    "right": 0,
  },
  "ignore_exclusive": false,
  "pinnable": true,
  "pin_with_key": true,
  "pin_key": 274, // run `way-edges` with `--mouse-debug`, then click on any widget to get the key printed in log
  "widget": {
    // this can be `btn`, `slider`, `wrap-box`, `workspace`.
    // the lsp completion might not show them all, but once you write it, the rest property completion should work
    "type": "btn",
    "thickness": 20,
    "length": "25%",
    "border_width": 5,
    "border_color": "#112233aa",
    "color": "#ffeeddaa",
    "event_map": {
      "272": "sh -c pkill nwg-drawer || nwg-drawer", // left click
      "273": "niri msg action maximize-column", // right click
      "274": "niri msg action close-window", // middle click
      "275": "niri msg action toggle-overview", // side click 1
      "276": "niri msg action toggle-column-tabbed-display", // side click 2
    },
  },
},
```

| Name                | Description                                                                   |
| ------------------- | ----------------------------------------------------------------------------- |
| name                | can be null, but in order to `toggle-pin` it, name is a must use              |
| edge                | monitors edge                                                                 |
| position            | Position on that edge                                                         |
| layer               | wlr layershell layer                                                          |
| monitor             | which monitor to spawn, can be multiple                                       |
| extra_trigger_size  | extra transparent area extened base on edge only for additional mouse trigger |
| preview_size        | extend the content out of the edge                                            |
| animation_curve     | linear, ease-expo...                                                          |
| transition_duration | ms to pop out                                                                 |
| margins             | margins.                                                                      |
| ignore_exclusive    | ignores the other layershell's exclusive zone, stick right on the edge        |
| pinnable            | able to pin the widget, pin will not auto hide the widget                     |
| pin_with_key        | whether use a mouse key to pin the widget, only works when pinnable=true      |
| pin_key             | the mouse key to pin the widget, only works when pin_with_key=true            |
| widget              | the actual widget configurations                                              |
