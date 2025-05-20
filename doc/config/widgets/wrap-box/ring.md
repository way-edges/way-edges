# Ring

```jsonc
{
  "index": [-1, -1], // position in the grid layout. -1 means next available position.
  "widget": {
    "type": "ring",
    "animation_curve": "ease-expo",
    "bg_color": "#00000000",
    "fg_color": "#00000000",
    "font_family": "serif",
    "font_size": 0,
    "prefix": "prefix {float:2,100}%",
    "suffix": "surfix {float:2,100}%",
    "prefix_hide": false,
    "suffix_hide": false,
    "ring_width": 20,
    "radius": 35,
    "text_transition_ms": 100, // ms
    // "preset": {
    //   "type": "ram",
    //   "update_interval": 1000, // ms
    // },
    // "preset": {
    //   "type": "battery",
    //   "update_interval": 1000, // ms
    // },
    // "preset": {
    //   "type": "cpu",
    //   "update_interval": 1000, // ms
    // },
    // "preset": {
    //   "type": "swap",
    //   "update_interval": 1000, // ms
    // },
    // "preset": {
    //   "type": "disk",
    //   "update_interval": 1000, // ms
    //   "partition": "/",
    // },
    "preset": {
      "type": "custom",
      "cmd": "echo -n 0.5", // this is the command to run. The command should output a number between 0 and 1.
      "update_interval": 1000, // ms
    },
  },
},
```

| Name               | Description                                                 |
| ------------------ | ----------------------------------------------------------- |
| type               | const `ring`                                                |
| animation_curve    | animation curve                                             |
| bg_color           | color                                                       |
| fg_color           | color                                                       |
| font_family        | font family                                                 |
| font_size          | font size                                                   |
| prefix             | text template                                               |
| suffix             | text template                                               |
| prefix_hide        | bool                                                        |
| suffix_hide        | bool                                                        |
| ring_width         | int                                                         |
| radius             | total radius of the circle                                  |
| text_transition_ms | ms                                                          |
| preset             | `ram` or `battery` or `cpu` or `swap` or `disk` or `custom` |

## Preset: ram

```jsonc
"preset": {
  "type": "ram",
  "update_interval": 1000, // ms
},
```

| Name            | Description |
| --------------- | ----------- |
| type            | const `ram` |
| update_interval | ms          |

## Preset: battery

```jsonc
"preset": {
  "type": "battery",
  "update_interval": 1000, // ms
},
```

| Name            | Description     |
| --------------- | --------------- |
| type            | const `battery` |
| update_interval | ms              |

## Preset: cpu

```jsonc
"preset": {
  "type": "cpu",
  "update_interval": 1000, // ms
  "core": 0, // null for all cores
},
```

| Name            | Description        |
| --------------- | ------------------ |
| type            | const `cpu`        |
| update_interval | ms                 |
| core            | null for all cores |

## Preset: swap

```jsonc
"preset": {
  "type": "swap",
  "update_interval": 1000, // ms
},
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `swap` |
| update_interval | ms           |

## Preset: disk

```jsonc
"preset": {
  "type": "disk",
  "update_interval": 1000, // ms
  "partition": "/",
},
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `disk` |
| update_interval | ms           |
| partition       | partition    |

## Preset: custom

```jsonc
"preset": {
  "type": "custom",
  "cmd": "echo -n 0.5", // this is the command to run. The command should output a number between 0 and 1.
  "update_interval": 1000, // ms
},
```

| Name            | Description                                                                     |
| --------------- | ------------------------------------------------------------------------------- |
| type            | const `custom`                                                                  |
| cmd             | this is the command to run. The command should output a number between 0 and 1. |
| update_interval | ms                                                                              |
