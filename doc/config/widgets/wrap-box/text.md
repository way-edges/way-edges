# Text

```json
{
  "type": "text",
  "fg-color": "#00000000",
  "font-family": "monospace",
  "font-size": 24,
  "event-map": {
    // same as btn
  },
  // "preset": {
  //   "type": "time",
  //   "format": "%Y-%m-%d %H:%M:%S",
  //   "time-zone": "uk", // null for local time
  //   "update-interval": 1000, // ms
  // },
  "preset": {
    "type": "custom",
    "cmd": "echo -n aaa", // this is the command to run. The command should output a string.
    "update-interval": 1000, // ms
  },
},
```

| Name        | Description        |
| ----------- | ------------------ |
| type        | const `text`       |
| fg-color    | color              |
| font-family | font family        |
| font-size   | font size          |
| event-map   | same as button     |
| preset      | `time` or `custom` |

## Preset: time

```json
"preset": {
  "type": "time",
  "format": "%Y-%m-%d %H:%M:%S",
  "time-zone": "uk", // null for local time
  "update-interval": 1000, // ms
},
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `time` |
| format          | time format  |
| time-zone       | time zone    |
| update-interval | ms           |

## Preset: custom

```json
"preset": {
  "type": "custom",
  "cmd": "echo -n aaa", // this is the command to run. The command should output a string.
  "update-interval": 1000, // ms
},
```

| Name            | Description                                                     |
| --------------- | --------------------------------------------------------------- |
| type            | const `custom`                                                  |
| cmd             | this is the command to run. The command should output a string. |
| update-interval | ms                                                              |
