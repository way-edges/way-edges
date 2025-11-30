# 💻 Way-Edges

# Important

- fix: clear old contents before reloading widgets, is won't stick on the screen anymore.
- fix: correctly handles outputs update event, and reloads widgets accordingly
- feat: allow `mouse-xxx` for specifying some mouse events in `event-map`, in #155 by @SheffeyG
  - including: `mouse-left`, `mouse-right`, `mouse-middle`, `mouse-side`, `mouse-extra`, `mouse-forward`, `mouse-back`.
- feat: introduce `offset` property for widgets, which pushes the widget further out from the edge. in #154 by @SheffeyG

# Others

- feat: watching configuration file directly instead of the directory containing it, supports symlinks.
- feat: detach shell commands so that it still exists after way-edges exits
- fix: write niri Event ourselves incase something breaks after new changes were made in niri-git
- feat: reload widgets only when idle
- feat: round all corners for `wrap-box` when offset is used. in #156 by @SheffeyG
- feat: enable tokio_iouring. Nix supports: #148 by @Brisingr05
- nix: flake.lock: Update. in #153 by @oliviafloof
- doc: add an animation showing how we make the widgets pop, and how the coordinates are calculated, incase i forgor again 💀
- chore: send desktop notifications no more, use terminal output only
- chore: optimize build flags for faster compilation
- chore: remove unused dependencies
