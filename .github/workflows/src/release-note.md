# 💻 Way-Edges

## KDL configuration

We now have kdl configuration support, which is a cleaner and a more human-friendly configuration format.

> [!NOTE]
> The old json configuration is still supported,  
> but we recommend switching to kdl for better readability and maintainability.

I personally have switched to kdl for my own configuration, check it [here](https://github.com/ogios/dots/blob/master/way-edges/config.kdl).

## Breaking changes

- `workspace.niri.filter-empty` now renamed to `workspace.niri.preserve-empty`
- `workspace.niri.preserve-empty` is now preserving named workspaces even if they are empty.
- use `kc-` prefix with bit number support for event map, now you can use something like `kc-0x110` for the key.
- all the boolean fields are now `false` by default, which means for settings like `pinnable` should now be set explicitly.

## Other changes

- fix race condition for tray widget, avoid another possible crash
- add event map for ring&text widget
- make the `monitor` setting more robust. accepting multiple string and number at the same time.
- less log spam
