# Slider

```jsonc
{
  // ... other basic configs omitted here for brevity
  "widget": {
    "type": "slider",
    "thickness": 20,
    "length": "25%",
    "border_width": 3,
    "border_color": "#112233aa",
    "fg_color": "#ffeeddaa",
    "bg_color": "#112233aa",
    "bg_text_color": "#124123aa",
    "fg_text_color": "#124123aa",
    "redraw_only_on_internal_update": true, // This is when you want to reduce the cpu usage. The progress update by manually dragging the slider is sent, but it won't be redrawn until the value is changed by other means.
    "radius": 20, // corner radius
    "obtuse_angle": 120, // in degrees(90~180). controls how much curve the widget has
    // "preset": {
    //   "type": "custom",
    //   "interval_update": [100, "echo -n 0.1"], // update the progress. The first value is the interval in ms, and the second value is the command to run. The command should output a number between 0 and 1.
    //   "on_change": "notify-send {float:2,100}%", // this is the command to run when the value changes. The value is passed as a parameter. You can use {float:2,100} to format the value as a float with 2 decimal places multiplied by 100.
    //   "event_map": {
    //     // same as btn
    //   },
    // },
    // "preset": {
    //   "type": "speaker",
    //   "type": "microphone",
    //   "device": "alsa_output.pci-0000_00_1f.3.analog-stereo", // Name of the device, not description of the device. null for default sink/source
    //   "animation_curve": "ease-expo", // mute animation
    //   "mute_text_color": "#00000000",
    //   "mute_color": "#00000000",
    // },
    "preset": {
      "type": "backlight",
      "device": "nvidia_0", // this is the name of the device. Find it under `/sys/class/backlight/` It should be something like `nvidia_0`, `intel_0`, etc.
    },
  },
},
```

| Name                           | Description                                                                                                                                                                    |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| type                           | const `slider`                                                                                                                                                                 |
| thickness                      | can be relative(`xx%`) or a int number                                                                                                                                         |
| length                         | can be relative(`xx%`) or a int number                                                                                                                                         |
| border_width                   | int                                                                                                                                                                            |
| fg_color                       | hex only, but with alpha channel supported                                                                                                                                     |
| bg_color                       | hex only, but with alpha channel supported                                                                                                                                     |
| fg_text_color                  | hex only, but with alpha channel supported                                                                                                                                     |
| bg_text_color                  | hex only, but with alpha channel supported                                                                                                                                     |
| border_color                   | hex only, but with alpha channel supported                                                                                                                                     |
| redraw_only_on_internal_update | This is when you want to reduce the cpu usage. The progress update by manually dragging the slider is sent, but it won't be redrawn until the value is changed by other means. |
| radius                         | corner radius                                                                                                                                                                  |
| obtuse_angle                   | in degrees(90~180). controls how much curve the widget has                                                                                                                     |
| preset                         | 4 presets: `custom`, `speaker`, `microphone`, `backlight`                                                                                                                      |

## Preset: Custom

```jsonc
"preset": {
  "type": "custom",
  "interval_update": [100, "echo -n 0.1"], // update the progress. The first value is the interval in ms, and the second value is the command to run. The command should output a number between 0 and 1.
  "on_change": "notify-send {float:2,100}%", // this is the command to run when the value changes. The value is passed as a parameter. You can use {float:2,100} to format the value as a float with 2 decimal places multiplied by 100.
  "event_map": {
    // same as btn
  },
},
```

| Name            | Description                                                                                                                                                                              |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| type            | const `custom`                                                                                                                                                                           |
| interval_update | update the progress. The first value is the interval in ms, and the second value is the command to run. The command should output a number between 0 and 1.                              |
| on_change       | this is the command to run when the value changes. The value is passed as a parameter. You can use {float:2,100} to format the value as a float with 2 decimal places multiplied by 100. |
| event_map       | same as button                                                                                                                                                                           |

## Preset: speaker/microphone

```jsonc
"preset": {
  "type": "speaker",
  // "type": "microphone",
  "device": "alsa_output.pci-0000_00_1f.3.analog-stereo", // Name of the device, not description of the device. null for default sink/source
  "animation_curve": "ease-expo", // mute animation
  "mute_text_color": "#00000000",
  "mute_color": "#00000000",
},
```

| Name            | Description                                                                     |
| --------------- | ------------------------------------------------------------------------------- |
| type            | const `speaker` or const `microphone`                                           |
| device          | Name of the device, not description of the device. null for default sink/source |
| animation_curve | mute animation                                                                  |
| mute_text_color | color                                                                           |
| mute_color      | color                                                                           |

## Preset: backlight

```jsonc
"preset": {
  "type": "backlight",
  "device": "nvidia_0", // this is the name of the device. Find it under `/sys/class/backlight/` It should be something like `nvidia_0`, `intel_0`, etc.
},
```

| Name   | Description                                                                                                                   |
| ------ | ----------------------------------------------------------------------------------------------------------------------------- |
| type   | const `speaker` or const `microphone`                                                                                         |
| device | this is the name of the device. Find it under `/sys/class/backlight/` It should be something like `nvidia_0`, `intel_0`, etc. |
