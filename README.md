<!-- https://github.com/user-attachments/assets/37d267cb-1cb4-44b4-81a8-8ac02bb741cb -->



https://github.com/user-attachments/assets/e4b00c74-f4b1-4e45-9ff1-79f1b8bc6e13



## Doc

Please refer to [https://way-edges.github.io/description]

## Installation

### Arch(aur)

- [way-edges-bin](https://aur.archlinux.org/packages/way-edges-bin)
- [way-edges-git](https://aur.archlinux.org/packages/way-edges-git)

### Manual

```shell
git clone https://github.com/way-edges/way-edges.git
cd way-edges && cargo build --release
```

## Configuration

Place `config.jsonc` under `~/.config/way-edges/`

### Full doc

Doc: https://way-edges.github.io/basic_config

### Schema

Please refer to [config.jsonc](./config/config.jsonc) and [schema](./config/config.schema.json)

## Launch

1. Run daemon first(`way-edges daemon`).
2. Add group of widgets given group name specified in your configuration file(`way-edges add <group_name>`).
3. Some command require widget_name specified in order to operate.

```rust
Hidden widget on the screen edges

Usage: way-edges [OPTIONS] <COMMAND>

Commands:
  daemon     run daemon. There can only be one daemon at a time
  add        add group of widgets to applicatoin given group name
  rm         remove group of widgets to applicatoin given group name
  togglepin  toggle pin of a widget under certain group. format: <group_name>:<widget_name>
  quit       close daemon
  help       Print this message or the help of the given subcommand(s)

Options:
  -d, --mouse-debug  whether enable mouse click output, shoule be used width daemon command
  -h, --help         Print help
  -V, --version      Print version
```

<details>

  <summary>

## TODO

  </summary>

- [x] fixed FPS.
- [x] widget grouping
- [x] configuration file
- [x] JSON schema for configuration file
- [x] watch file & hot reload
- [x] CLI
- [x] monitor relative height
- [ ] wayland working area relative height (wayland compositor specific relative height as features(including exclusive zone calculation))
- [ ] ease-in & ease-out widget motion curve
- [ ] widgets
  - [x] Button
  - [x] Slider
  - [x] PulseAudio(Speaker, Microphone)
  - [x] Brightness
  - [x] Ring progress(ram/swap/cpu/battery/disk/custom)
  - [x] Text(time/custom)
  - [x] Box
  - [x] Hyprland Workspaces
  - [ ] Tray
- [ ] ?multiple click & long press & release event(Button widget)
- [ ] ~~?buttom size calculation, arrangement and overlap(should this be considered?)~~

</details>
