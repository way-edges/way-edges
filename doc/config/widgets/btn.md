# Button

```json
{
"type": "btn",
"thickness": 20,
"length": "25%",
"border-width": 5,
"border-color": "#112233aa",
"color": "#ffeeddaa",
"event-map": {
  "mouse-left": "sh -c pkill nwg-drawer || nwg-drawer -ovl",
  "mouse-right": "niri msg action maximize-column",
  "mouse-middle": "niri msg action close-window",
  "mouse-side": "niri msg action toggle-overview",
  "mouse-extra": "niri msg action toggle-column-tabbed-display",
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
