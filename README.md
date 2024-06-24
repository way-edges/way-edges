[show_case](./gif/2024-06-2423-18-58-ezgif.com-video-to-gif-converter.gif)

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
- [ ] watch file & hot reload
- [ ] ease-in & ease-out button motion curve
- [ ] long press & release event
- [ ] ?relative height(useful but is it possible for wayland client?)
  - check if on_size_change signal works
  - get available working area(how?)
  - every draw process should be initialized within draw func, record and compare height in each draw call. (lots of code rewrite)
- [ ] ?buttom size calculation, arrangement and overlap(should this be considered?)
- [ ] ?hover event(only bind with transition, not necessary - for now)
- [ ] ?button click effects optimization(gradience)

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
