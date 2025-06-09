# Button

```jsonc
{
"type": "btn",
"thickness": 20,
"length": "25%",
"border-width": 5,
"border-color": "#112233aa",
"color": "#ffeeddaa",
"event-map": {
  "272": "sh -c pkill nwg-drawer || nwg-drawer", // left click
  "273": "niri msg action maximize-column", // right click
  "274": "niri msg action close-window", // middle click
  "275": "niri msg action toggle-overview", // side click 1
  "276": "niri msg action toggle-column-tabbed-display", // side click 2
}
```

| Name         | Description                                                                                                         |
| ------------ | ------------------------------------------------------------------------------------------------------------------- |
| type         | const `btn`                                                                                                         |
| thickness    | can be relative(`xx%`) or a int number                                                                              |
| length       | can be relative(`xx%`) or a int number                                                                              |
| border-width | int                                                                                                                 |
| color        | hex only, but with alpha channel supported                                                                          |
| border-color | hex only, but with alpha channel supported                                                                          |
| event-map    | each mouse button match a shell command, launch program with `--mouse-debug` and click on the widget to see the key |
