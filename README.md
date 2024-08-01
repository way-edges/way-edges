

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
- [ ] CLI
- [ ] ?multiple click & long press & release event(Buttom widget)
- [ ] ?hover event(only bind with transition, not necessary - for now)
- [ ] ?button click effects optimization(gradience)
- [ ] ~~?buttom size calculation, arrangement and overlap(should this be considered?)~~

## Configuration

Please refer to [config.jsonc](./config/config.jsonc) and [schema](./config/config.schema.json)

Place `config.jsonc` under `~/.config/way-edges/`

## Arguments

```rust
Usage: gtk4-test [OPTIONS] [GROUP]

Arguments:
  [GROUP]  which grouop to activate

Options:
  -d, --mouse-debug  whether enable mouse click output
  -h, --help         Print help
  -V, --version      Print version
```
