# 💻 Way-Edges

- break: slider custom-preset:
  - interval-update -> update-interval and update-command
  - on-change -> on-change-command

- nix: change nixpkgs flake input to nixos-25.05 + very minor refactor. #139 @Brisingr05

## Hyprland workspace should be stable now

- Hyprland empty workspace no longer get excluded
- Ignore Hyprland special workspace

## Tray should be stable now

- Able to update tray icon with pixmap data
- implement tray menu diff event

## New scroll support for certain widgets

- Mouse/Touchpad scroll support for slider
- Mouse scroll support for workspace

## Other

- border-width and border-radius for workspace
- `pin-on-startup` option
- Move socket file from /tmp to XDG_RUNTIME_DIR
