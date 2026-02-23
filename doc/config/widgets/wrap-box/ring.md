# Ring

```json
{
  "index": [-1, -1], // position in the grid layout. -1 means next available position.
  "type": "ring",
  "animation-curve": "ease-expo",
  "bg-color": "#00000000",
  "fg-color": "#00000000",
  "font-family": "serif",
  "font-size": 0,
  "prefix": "prefix {float:2,100}%",
  "suffix": "surfix {float:2,100}%",
  "prefix-hide": false,
  "suffix-hide": false,
  "ring-width": 20,
  "radius": 35,
  "text-transition-ms": 100, // ms
  "event-map": {
    // same as btn
  },
  // "preset": {
  //   "type": "ram",
  //   "update-interval": 1000, // ms
  // },
  // "preset": {
  //   "type": "battery",
  //   "update-interval": 1000, // ms
  // },
  // "preset": {
  //   "type": "cpu",
  //   "update-interval": 1000, // ms
  // },
  // "preset": {
  //   "type": "swap",
  //   "update-interval": 1000, // ms
  // },
  // "preset": {
  //   "type": "disk",
  //   "update-interval": 1000, // ms
  //   "partition": "/",
  // },
  "preset": {
    "type": "custom",
    "cmd": "echo -n 0.5", // this is the command to run. The command should output a number between 0 and 1.
    "update-interval": 1000, // ms
  },
},
```

| Name               | Description                                                 |
| ------------------ | ----------------------------------------------------------- |
| type               | const `ring`                                                |
| animation-curve    | animation curve                                             |
| bg-color           | color                                                       |
| fg-color           | color                                                       |
| font-family        | font family                                                 |
| font-size          | font size                                                   |
| prefix             | text template                                               |
| suffix             | text template                                               |
| prefix-hide        | bool                                                        |
| suffix-hide        | bool                                                        |
| ring-width         | int                                                         |
| radius             | total radius of the circle                                  |
| text-transition-ms | ms                                                          |
| event-map          | same as button                                              |
| preset             | `ram` or `battery` or `cpu` or `swap` or `disk` or `custom` |

## Preset: ram

```jsonc
"preset": {
  "type": "ram",
  "update-interval": 1000, // ms
},
```

| Name            | Description |
| --------------- | ----------- |
| type            | const `ram` |
| update-interval | ms          |

## Preset: battery

```jsonc
"preset": {
  "type": "battery",
  "update-interval": 1000, // ms
},
```

| Name            | Description     |
| --------------- | --------------- |
| type            | const `battery` |
| update-interval | ms              |

## Preset: cpu

```jsonc
"preset": {
  "type": "cpu",
  "update-interval": 1000, // ms
  "core": 0, // null for all cores
},
```

| Name            | Description        |
| --------------- | ------------------ |
| type            | const `cpu`        |
| update-interval | ms                 |
| core            | null for all cores |

## Preset: swap

```jsonc
"preset": {
  "type": "swap",
  "update-interval": 1000, // ms
},
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `swap` |
| update-interval | ms           |

## Preset: disk

```jsonc
"preset": {
  "type": "disk",
  "update-interval": 1000, // ms
  "partition": "/",
},
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `disk` |
| update-interval | ms           |
| partition       | partition    |

## Preset: custom

```jsonc
"preset": {
  "type": "custom",
  "cmd": "echo -n 0.5", // this is the command to run. The command should output a number between 0 and 1.
  "update-interval": 1000, // ms
},
```

| Name            | Description                                                                     |
| --------------- | ------------------------------------------------------------------------------- |
| type            | const `custom`                                                                  |
| cmd             | this is the command to run. The command should output a number between 0 and 1. |
| update-interval | ms                                                                              |
