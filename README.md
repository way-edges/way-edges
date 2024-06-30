
https://github.com/ogios/way-edges/assets/96933655/98219132-b37e-4d8d-9b4a-01c64105e25e


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
- [x] relative height
  - ~~check if on_size_change signal works~~ no need.
  - get available working area(~~how?~~ each compositor specific)
  - ~~every draw process should be initialized within draw func, record and compare height in each draw call. (lots of code rewrite)~~ no need.
- [x] wayland compositor specific relative height as features(including exclusive zone calculation)
- [ ] watch file & hot reload
- [ ] ease-in & ease-out button motion curve
- [ ] modulize mouse event
- [ ] multiple click & long press & release event
- [ ] add some customized widgets
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
