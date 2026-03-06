# Configuration

- [Root](root.md)
- [Widget](widget.md)
  - [Workspace](widgets/workspace.md)
  - [Button](widgets/btn.md)
  - [Slider](widgets/slider.md)
  - [WrapBox](widgets/wrap-box/wrap-box.md)
    - [Ring](widgets/wrap-box/ring.md)
    - [Text](widgets/wrap-box/text.md)
    - [Tray](widgets/wrap-box/tray.md)

## KDL

I'm trying to migrate to KDL, currently it's still in the early stages.  
I recommend using KDL, it is much more clean, and comes with the better readability of course.  
Tough it doesn't support schema yet.

Create a `config.kdl` under `~/.config/way-edges/`.

## JSON

Create a `config.json` under `~/.config/way-edges/`.

Here we explain all the configurations

Add a `$schema` to the root of the configuration to get auto-completions from your IDE:

```json
{
  "$schema": "./schema.json"
}
```

You can checkout [all_in_one.jsonc](all_in_one.jsonc) for a complete example.
