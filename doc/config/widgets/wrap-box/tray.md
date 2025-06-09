# Tray

```jsonc
"widget": {
  "type": "tray",
  "font-family": "monospace",
  "grid-align": "bottom-center", // 9 positions: center-left, center-right, top-left, top-right, bottom-left, bottom-right, left-top, left-bottom, right-top, right-bottom
  "icon-theme": "breeze", // null will fetch the default icon theme
  "icon-size": 20,
  "tray-gap": 2,
  "header-draw-config": {
    "text-color": "#00000000",
    "font-pixel-height": 20,
  },
  // "header-menu-align": "left"
  "header-menu-align": "right",
  // "header-menu-stack": "header-top",
  "header-menu-stack": "menu-top",
  "menu-draw-config": {
    "border-color": "#00000000",
    "text-color": "#00000000",
    "marker-color": "#00000000",
    "font-pixel-height": 22,
    "icon-size": 20,
    "marker-size": 20,
    "separator-height": 5,
    "margin": [12, 12], // horizontal, vertical
  },
},
```

| Name               | Description                                                                                                                            |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| type               | const `tray`                                                                                                                           |
| font-family        | font family                                                                                                                            |
| grid-align         | 9 positions: center-left, center-right, top-left, top-right, bottom-left, bottom-right, left-top, left-bottom, right-top, right-bottom |
| icon-theme         | null will fetch the default icon theme                                                                                                 |
| icon-size          | int                                                                                                                                    |
| tray-gap           | int                                                                                                                                    |
| header-draw-config |                                                                                                                                        |
| header-menu-align  | left or right                                                                                                                          |
| header-menu-stack  | header-top or menu-top                                                                                                                 |
| menu-draw-config   |                                                                                                                                        |

## header-draw-config

| Name              | Description |
| ----------------- | ----------- |
| text-color        | color       |
| font-pixel-height | int         |

## menu-draw-config

| Name              | Description            |
| ----------------- | ---------------------- |
| border-color      | color                  |
| text-color        | color                  |
| marker-color      | color or null          |
| font-pixel-height | int                    |
| icon-size         | int                    |
| marker-size       | int                    |
| separator-height  | int                    |
| margin            | [horizontal, vertical] |
