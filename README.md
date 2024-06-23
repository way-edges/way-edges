## TODO

- [x] Cairo paint buttons & shadow
- [x] GTK4 DrawArea & pre-draw surface cache
- [x] click event
- [x] button movement motion curve(linear for now)
- [x] Frame rate management, only render when content visible, saves some resources
- [x] `wl_surface` input region change with button movement
- [x] pre-draw surface transform to fit other edges
- [x] widget grouping
- [x] configurations
- [ ] cmdline args
- [ ] watch file & hot reload
- [ ] ease-in & ease-out button motion curve
- [ ] ?relative height(maybe useful but don't know if it's possible)
  - check if on size change signal works when other window appears
  - if use relative height, every draw process should be initialized within draw func, record and compare height in each draw call
- [ ] ?size calculation and buttons overlap(should this be considered?)
- [ ] ?hover event(only bind with transition now, not certain if it's needed)
- [ ] ?button click effects optimization
