# Widget

```jsonc
{
  "namespace": "widget_base_example",
  "edge": "top",
  "position": "left",
  "layer": "overlay",
  // "monitor": 0,
  // "monitor": "*",
  // "monitor": "eDP-1",
  "monitor": ["eDP-1", "HDMI-A-1"],
  "extra-trigger-size": 1, // or "10%"
  "preview-size": 20, // or "100%"
  "animation-curve": "ease-expo",
  "transition-duration": 300,
  "margins": {
    "top": 0,
    "left": 0,
    "bottom": 0,
    "right": 0,
  },
  "ignore-exclusive": false,
  "pinnable": true,
  "pin-on-startup": false,
  "pin-with-key": true,
  "pin-key": 274, // run `way-edges` with `--mouse-debug`, then click on any widget to get the key printed in log

  // NOTE: THE REST OF THESE CONFIGURATIONS ARE ENUM SPECIFIC
  // `type` can be `btn`, `slider`, `wrap-box`, `workspace`.
  // the lsp completion might not show them all, but once you write it, the rest property completion should work
  "type": "btn",
  // ...
  // "thickness": 20,
  // "length": "25%",
  // "border-width": 5,
  // "border-color": "#112233aa",
  // "color": "#ffeeddaa",
  // "event-map": {
  //   "272": "sh -c pkill nwg-drawer || nwg-drawer", // left click
  //   "273": "niri msg action maximize-column", // right click
  //   "274": "niri msg action close-window", // middle click
  //   "275": "niri msg action toggle-overview", // side click 1
  //   "276": "niri msg action toggle-column-tabbed-display", // side click 2
  // },
},
```

| Name                | Description                                                                   |
| ------------------- | ----------------------------------------------------------------------------- |
| namespace           | can be null, but in order to `togglepin` it you have to specify this          |
| edge                | monitors edge                                                                 |
| position            | Position on that edge                                                         |
| layer               | wlr layershell layer                                                          |
| monitor             | which monitor to spawn, can be multiple                                       |
| extra-trigger-size  | extra transparent area extened base on edge only for additional mouse trigger |
| preview-size        | extend the content out of the edge                                            |
| animation-curve     | linear, ease-expo...                                                          |
| transition-duration | ms to pop out                                                                 |
| margins             | margins.                                                                      |
| ignore-exclusive    | ignores the other layershell's exclusive zone, stick right on the edge        |
| pinnable            | able to pin the widget, pin will not auto hide the widget                     |
| pin-on-startup      | widget start with pin, works only if pinnable=true state                      |
| pin-with-key        | whether use a mouse key to pin the widget, only works when pinnable=true      |
| pin-key             | the mouse key to pin the widget, only works when pin-with-key=true            |
| type                | can be `btn`, `slider`, `wrap-box`, `workspace`                               |
