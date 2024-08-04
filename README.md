https://github.com/user-attachments/assets/37d267cb-1cb4-44b4-81a8-8ac02bb741cb

## TODO

- [x] Cairo paint buttons & shadow
- [x] GTK4 DrawArea & pre-draw surface cache
- [x] click event
- [x] button movement: motion curve(linear for now: y=x)
- [x] Frame rate management, only renders when visible.(to save resources)
- [x] `wl_surface` input region dynamically changes with button movement
- [x] pre-draw surface transformation(to fit other edges)
- [x] widget grouping
- [x] configuration file
- [x] cmdline args
- [x] margin
- [x] watch file & hot reload
- [x] modulize mouse event
- [-] relative height(wayland compositor specific relative height as features(including exclusive zone calculation))
- [x] CLI
- [ ] ease-in & ease-out button motion curve
- [ ] widgets
  - [x] Button
  - [x] Slider(for volume, brightness, etc.)
  - [x] PulseAudio(Speaker, Microphone)
    - [x] allow specify device(only default for now)
  - [x] Brightness
  - [x] Ring progress(for cpu/ram... status)
  - [x] Box
  - [ ] Time
  - [ ] Tray
  - [ ] Hyprland Workspaces
- [ ] ?multiple click & long press & release event(Button widget)
- [ ] ~~?buttom size calculation, arrangement and overlap(should this be considered?)~~

## Configuration

Please refer to [config.jsonc](./config/config.jsonc) and [schema](./config/config.schema.json)

Place `config.jsonc` under `~/.config/way-edges/`

## Arguments

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
