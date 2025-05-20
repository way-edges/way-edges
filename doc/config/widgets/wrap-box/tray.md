# Tray

```jsonc
"widget": {
  "type": "tray",
  "font_family": "monospace",
  "grid_align": "bottom_center", // 9 positions: center_left, center_right, top_left, top_right, bottom_left, bottom_right, left_top, left_bottom, right_top, right_bottom
  "icon_theme": "breeze", // null will fetch the default icon theme
  "icon_size": 20,
  "tray_gap": 2,
  "header_draw_config": {
    "text_color": "#00000000",
    "font_pixel_height": 20,
  },
  // "header_menu_align": "left"
  "header_menu_align": "right",
  // "header_menu_stack": "header_top",
  "header_menu_stack": "menu_top",
  "menu_draw_config": {
    "border_color": "#00000000",
    "text_color": "#00000000",
    "marker_color": "#00000000",
    "font_pixel_height": 22,
    "icon_size": 20,
    "marker_size": 20,
    "separator_height": 5,
    "margin": [12, 12], // horizontal, vertical
  },
},
```

| Name               | Description                                                                                                                            |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| type               | const `tray`                                                                                                                           |
| font_family        | font family                                                                                                                            |
| grid_align         | 9 positions: center_left, center_right, top_left, top_right, bottom_left, bottom_right, left_top, left_bottom, right_top, right_bottom |
| icon_theme         | null will fetch the default icon theme                                                                                                 |
| icon_size          | int                                                                                                                                    |
| tray_gap           | int                                                                                                                                    |
| header_draw_config |                                                                                                                                        |
| header_menu_align  | left or right                                                                                                                          |
| header_menu_stack  | header_top or menu_top                                                                                                                 |
| menu_draw_config   |                                                                                                                                        |

## header_draw_config

| Name              | Description |
| ----------------- | ----------- |
| text_color        | color       |
| font_pixel_height | int         |

## menu_draw_config

| Name              | Description            |
| ----------------- | ---------------------- |
| border_color      | color                  |
| text_color        | color                  |
| marker_color      | color or null          |
| font_pixel_height | int                    |
| icon_size         | int                    |
| marker_size       | int                    |
| separator_height  | int                    |
| margin            | [horizontal, vertical] |
