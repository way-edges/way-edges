# Text

```jsonc
{
"widget": {
  "type": "text",
  "fg_color": "#00000000",
  "font_family": "monospace",
  "font_size": 24,
  // "preset": {
  //   "type": "time",
  //   "format": "%Y-%m-%d %H:%M:%S",
  //   "time_zone": "uk", // null for local time
  //   "update_interval": 1000, // ms
  // },
  "preset": {
    "type": "custom",
    "update_with_interval_ms": [1000, "echo -n aaa"], // this is the command to run. The command should output a string.
  },
},
```

| Name        | Description        |
| ----------- | ------------------ |
| type        | const `text`       |
| fg_color    | color              |
| font_family | font family        |
| font_size   | font size          |
| preset      | `time` or `custom` |

## Preset: time

```jsonc
"preset": {
  "type": "time",
  "format": "%Y-%m-%d %H:%M:%S",
  "time_zone": "uk", // null for local time
  "update_interval": 1000, // ms
},
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `time` |
| format          | time format  |
| time_zone       | time zone    |
| update_interval | ms           |

## Preset: custom

```jsonc
"preset": {
  "type": "custom",
  "update_with_interval_ms": [1000, "echo -n aaa"], // this is the command to run. The command should output a string.
},
```

| Name                    | Description                        |
| ----------------------- | ---------------------------------- |
| type                    | const `custom`                     |
| update_with_interval_ms | \[interval in ms, command to run\] |
