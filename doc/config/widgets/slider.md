# Slider

```jsonc
{
  // ... other basic configs omitted here for brevity
  "widget": {
    "type": "slider",
    "thickness": 20,
    "length": "25%",
    "border-width": 3,
    "border-color": "#112233aa",
    "fg-color": "#ffeeddaa",
    "bg-color": "#112233aa",
    "bg-text-color": "#124123aa",
    "fg-text-color": "#124123aa",
    "redraw-only-on-internal-update": true, // This is when you want to reduce the cpu usage. The progress update by manually dragging the slider is sent, but it won't be redrawn until the value is changed by other means.
    "radius": 20, // corner radius
    "obtuse-angle": 120, // in degrees(90~180). controls how much curve the widget has
    "scroll-unit": 0.005,
    // "preset": {
    //   "type": "custom",
    //   "update-interval": 100, // ms to execute update command
    //   "update-command": "echo -n 0.1", // The command should output a number between 0 and 1.
    //   "on-change-command": "notify-send {float:2,100}%", // this is the command to run when the value changes. The value is passed as a parameter. You can use {float:2,100} to format the value as a float with 2 decimal places multiplied by 100.
    //   "event-map": {
    //     // same as btn
    //   },
    // },
    // "preset": {
    //   "type": "speaker",
    //   "type": "microphone",
    //   "device": "alsa_output.pci-0000_00_1f.3.analog-stereo", // Name of the device, not description of the device. null for default sink/source
    //   "animation-curve": "ease-expo", // mute animation
    //   "mute-text-color": "#00000000",
    //   "mute-color": "#00000000",
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
| border-width                   | int                                                                                                                                                                            |
| fg-color                       | hex only, but with alpha channel supported                                                                                                                                     |
| bg-color                       | hex only, but with alpha channel supported                                                                                                                                     |
| fg-text-color                  | hex only, but with alpha channel supported                                                                                                                                     |
| bg-text-color                  | hex only, but with alpha channel supported                                                                                                                                     |
| border-color                   | hex only, but with alpha channel supported                                                                                                                                     |
| redraw-only-on-internal-update | This is when you want to reduce the cpu usage. The progress update by manually dragging the slider is sent, but it won't be redrawn until the value is changed by other means. |
| scroll-unit                    | 0 to 1. defines how much progress to change on 1 pixel vertical scroll from mouse wheel. default 0.005                                                                         |
| radius                         | corner radius                                                                                                                                                                  |
| obtuse-angle                   | in degrees(90~180). controls how much curve the widget has                                                                                                                     |
| preset                         | 4 presets: `custom`, `speaker`, `microphone`, `backlight`                                                                                                                      |

## Preset: Custom

```jsonc
"preset": {
  "type": "custom",
  "update-interval": 100, // ms to execute update command
  "update-command": "echo -n 0.1", // The command should output a number between 0 and 1.
  "on-change-command": "notify-send {float:2,100}%", // this is the command to run when the value changes. The value is passed as a parameter. You can use {float:2,100} to format the value as a float with 2 decimal places multiplied by 100.
  "event-map": {
    // same as btn
  },
},
```

| Name              | Description                                                                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| type              | const `custom`                                                                                                                                                                           |
| update-interval   | ms to execute update command                                                                                                                                                             |
| update-command    | The command should output a number between 0 and 1.                                                                                                                                      |
| on-change-command | this is the command to run when the value changes. The value is passed as a parameter. You can use {float:2,100} to format the value as a float with 2 decimal places multiplied by 100. |
| event-map         | same as button                                                                                                                                                                           |

## Preset: speaker/microphone

```jsonc
"preset": {
  "type": "speaker",
  // "type": "microphone",
  "device": "alsa_output.pci-0000_00_1f.3.analog-stereo", // Name of the device, not description of the device. null for default sink/source
  "animation-curve": "ease-expo", // mute animation
  "mute-text-color": "#00000000",
  "mute-color": "#00000000",
},
```

| Name            | Description                                                                     |
| --------------- | ------------------------------------------------------------------------------- |
| type            | const `speaker` or const `microphone`                                           |
| device          | Name of the device, not description of the device. null for default sink/source |
| animation-curve | mute animation                                                                  |
| mute-text-color | color                                                                           |
| mute-color      | color                                                                           |

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
