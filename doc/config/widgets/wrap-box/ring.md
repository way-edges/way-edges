# Ring

```kdl
item "ring" {
  index -1 -1 // position in the grid layout. -1 means next available position.
  animation-curve "ease-expo"
  bg-color "#00000000"
  fg-color "#00000000"
  font-family "serif"
  font-size 0
  prefix "prefix {float:2,100}%"
  suffix "surfix {float:2,100}%"
  prefix-hide
  suffix-hide
  ring-width 20
  radius 35
  text-transition-ms 100 // ms
  event-map {
    // same as btn
  }
  // preset "ram" {
  //   update-interval 1000 // ms
  // }
}
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

```kdl
preset "ram" {
  update-interval 1000 // ms
}
```

| Name            | Description |
| --------------- | ----------- |
| type            | const `ram` |
| update-interval | ms          |

## Preset: battery

```kdl
preset "battery" {
  update-interval 1000 // ms
}
```

| Name            | Description     |
| --------------- | --------------- |
| type            | const `battery` |
| update-interval | ms              |

## Preset: cpu

```kdl
preset "cpu" {
  update-interval 1000 // ms
  core 0 // null for all cores
}
```

| Name            | Description        |
| --------------- | ------------------ |
| type            | const `cpu`        |
| update-interval | ms                 |
| core            | null for all cores |

## Preset: swap

```kdl
preset "swap" {
  update-interval 1000 // ms
}
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `swap` |
| update-interval | ms           |

## Preset: disk

```kdl
preset "disk" {
  update-interval 1000 // ms
  partition "/" // partition to monitor, e.g. "/", "/home", "/mnt/data"
}
```

| Name            | Description  |
| --------------- | ------------ |
| type            | const `disk` |
| update-interval | ms           |
| partition       | partition    |

## Preset: custom

```kdl
preset "custom" {
  cmd "echo -n 0.5" // this is the command to run. The command should output a number between 0 and 1.
  update-interval 1000 // ms
}
```

| Name            | Description                                                                     |
| --------------- | ------------------------------------------------------------------------------- |
| type            | const `custom`                                                                  |
| cmd             | this is the command to run. The command should output a number between 0 and 1. |
| update-interval | ms                                                                              |
