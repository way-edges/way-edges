# Wrap Box

```json
{
  // ... other basic configs omitted here for brevity
  "widget": {
    "type": "wrap-box",
    "align": "center-left", // 9 positions: center-left, center-right, top-left, top-right, bottom-left, bottom-right, left-top, left-bottom, right-top, right-bottom
    "gap": 10,
    // "outlook": {
    //   "type": "window",
    //   "color": "#00000000",
    //   "border-radius": 5,
    //   "border-width": 15,
    //   "margins": {
    //     "left": 5,
    //     "right": 5,
    //     "bottom": 5,
    //     "top": 5,
    //   }
    // },
    "outlook": {
      "type": "board",
      "border-radius": 5,
      "color": "#00000000",
      "margins": {
        // ...
      },
    },
    "items": [
      {
        "index": [-1, -1], // position in the grid layout. -1 means next available position.
        "type": "ring",
        // ... ring configs omitted here for brevity
      },
    ],
  },
},
```

| Name    | Description                                                                                                                            |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| type    | const `wrap-box`                                                                                                                       |
| align   | 9 positions: center-left, center-right, top-left, top-right, bottom-left, bottom-right, left-top, left-bottom, right-top, right-bottom |
| gap     | gap between each widget                                                                                                                |
| outlook | `window` or `board`                                                                                                                    |
| items   | _**grid layout**_ widgets with each of their index and config                                                                          |

## Outlook: window

```jsonc
"outlook": {
  "type": "window",
  "color": "#00000000",
  "border-radius": 5,
  "border-width": 15,
  "margins": {
    "left": 5,
    "right": 5,
    "bottom": 5,
    "top": 5,
  }
},
```

| Name          | Description    |
| ------------- | -------------- |
| type          | const `window` |
| color         | color          |
| border-radius | int            |
| border-width  | int            |
| margins       | margins        |

## Outlook: board

```jsonc
"outlook": {
  "type": "board",
  "color": "#00000000",
  "border-radius": 5,
  "margins": {
    "left": 5,
    "right": 5,
    "bottom": 5,
    "top": 5,
  }
},
```

| Name          | Description   |
| ------------- | ------------- |
| type          | const `board` |
| color         | color         |
| border-radius | int           |
| margins       | margins       |

## items

| Name  | Description                                          |
| ----- | ---------------------------------------------------- |
| index | default \[-1, -1\], you can choose to leave it empty |

the rest of the widget config:

- [Ring](ring.md)
- [Text](text.md)
- [Tray](tray.md)
