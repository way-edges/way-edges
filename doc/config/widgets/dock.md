# Dock

The dock currently supports Niri. It shows open windows from all workspaces on
the widget's output, grouped in workspace order. `edge "top"` and `edge "bottom"`
use a horizontal layout; `edge "left"` and `edge "right"` use a vertical layout.

- Left-click a window to focus it.
- Middle-click a window to close it.
- Scroll to cycle through windows on that output.

A minimal standalone configuration is available in [dock.kdl](../dock.kdl).
Run it alongside your existing daemon from the repository root:

```sh
cargo build --release -p way-edges
./target/release/way-edges -i dock-test -c doc/config/dock.kdl
```

For a vertical dock, use [dock-left.kdl](../dock-left.kdl) or
[dock-right.kdl](../dock-right.kdl). Stop the current test instance before changing
configuration, then start it with the selected file:

```sh
./target/release/way-edges -i dock-test quit
./target/release/way-edges -i dock-test -c doc/config/dock-left.kdl
```

The only required setting is `edge "left"` or `edge "right"`. Windows stack from
top to bottom while workspace and window titles stay upright. The dock slides
out from the selected edge and uses the same `pop-duration` delay before hiding.
The locator and selection animation follow the vertical window order.

To stop that test instance, run `./target/release/way-edges -i dock-test quit`.
The example hides at the screen edge and appears on pointer hover. Add
`pin-on-startup` if you want it to stay visible.

`pop-duration` keeps the dock visible after the pointer leaves (1500ms by default).
Re-entering cancels the pending hide; leaving again starts a fresh interval.
This provides time to return to the dock when a focus change shrinks its surface
away from the pointer. Set `pop-duration 0` for immediate hiding.

To enlarge the dock, increase `windows.icon-size` (64 in the example).
`windows.font-size` controls window titles, and `workspaces.font-size` controls
workspace labels. Increase `windows.gap` for spacing between windows, and
`workspaces.gap` for spacing between workspace groups. Both sections also support
`margins` for inner padding. The examples use `windows.gap 0` and larger top/bottom
margins so adjacent window cells touch while their contents keep some breathing room.
When `windows.gap` is nonzero, the space between adjacent contents is included in
the neighboring window cells, so their hit areas still meet without dead zones.
`windows.hover-bg-color` controls the subtle background shown under the pointer
(default `#ffffff12`), including the item's padding.

Workspace headers sit above their window groups, with labels such as
`WS 02 / browser`. Each header has a rail spanning the windows it owns.
The focused workspace shows `FOCUS`; the focused window has an accent locator
and corner marks. `workspaces.active-fg-color` sets the locator accent.
`workspaces.rail-width` controls rail thickness (3 pixels by default).

Focus changes animate the outline's position and size together with the window
layout. `focus-transition-duration` sets the duration in milliseconds (260 by
default; 0 disables animation), and `focus-animation-curve` sets the easing curve
(`"ease-cubic"` by default in KDL). A focus change during a transition continues
from the displayed position. Completed transitions stop requesting frames.
Window titles stay upright in both orientations. Their actual text width sets
the selected window width, capped at `windows.title-width`; longer titles end
with an ellipsis. Workspace labels are measured at their full width.
Workspace separators are disabled by default; set `workspaces.separator-width`
to enable them.

```kdl
dock {
  namespace "dock"
  edge "bottom"
  monitor "*"
  pinnable
  pop-duration 1500
  color "#222"
  border-radius 10
  border-width 0
  margins {
    top 0
    left 0
    bottom 0
    right 0
  }
  workspaces {
    font-family "Geist Mono"
    font-size 26
    show-titles false
    fg-color "#555"
    bg-color "#00000000"
    active-fg-color "#777"
    active-bg-color "#00000000"
    border-color "#00000000"
    active-border-color "#00000000"
    border-width 1
    border-radius 10
    separator-width 1
    separator-margin 6
    separator-color "#444"
    gap 10
    margins {
      left 3
      right 3
      top 3
      bottom 3
    }
  }
  windows {
    font-family "Geist"
    font-size 16
    line-height 1.4
    title-width 80
    show-titles "focused"
    wrap-titles false
    icon-theme "Reversal"
    icon-size 32
    icon-fallback ""
    icon-opacity 1
    active-icon-opacity 1
    fg-color "#777"
    bg-color "#00000000"
    active-fg-color "#ccc"
    active-bg-color "#333"
    border-color "#00000000"
    active-border-color "#444"
    border-width 1
    border-radius 5
    separator-width 0
    separator-margin 6
    separator-color "#fff"
    gap 5
    margins {
      left 5
      right 5
      top 5
      bottom 5
    }
  }
}
```

| Name          | Description                                                                                                                            |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| grid-align    | top-left, top-center, top-right, center-left, center-center, center-right, bottom-left, bottom-center, bottom-right |
| color         | color                                                                                                                                  |
| border-color  | color                                                                                                                                  |
| border-width  | int                                                                                                                                    |
| border-radius | int                                                                                                                                    |
| margins       |                                                                                                                                        |

## workspaces

| Name                | Description |
| ------------------- | ----------- |
| font-family         | font family |
| font-size           | int |
| show-titles         | bool |
| fg-color            | color |
| active-fg-color     | color |
| bg-color            | color |
| active-bg-color     | color |
| border-color        | color |
| active-border-color | color |
| border-width        | int |
| border-radius       | int |
| separator-width     | int |
| separator-radius    | int |
| separator-margin    | int |
| separator-color     | color |
| gap                 | int |
| margins             | |


## windows

| Name                | Description |
| ------------------- | ----------- |
| font-family         | font family |
| font-size           | int |
| line-height         | float |
| title-width         | int |
| show-titles         | 4 options: always, never, focused, only (only option won't show icons)|
| wrap-titles         | bool |
| icon-theme          | string |
| icon-size           | int |
| icon-fallback       | path to image file or 1 char or null to fallback to the title first letter |
| icon-opacity        | float: 0.0 to 1.0 |
| active-icon-opacity | float: 0.0 to 1.0 |
| fg-color            | color |
| active-fg-color     | color |
| bg-color            | color |
| hover-bg-color      | color shown while hovering a window |
| active-bg-color     | color |
| border-color        | color |
| active-border-color | color |
| border-width        | int |
| border-radius       | int |
| separator-width     | int |
| separator-radius    | int |
| separator-margin    | int |
| separator-color     | color |
| gap                 | int |
| margins             | |
