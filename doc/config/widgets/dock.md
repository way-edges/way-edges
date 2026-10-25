# Dock

```kdl
dock {
  // commom widget config omitted
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
    grid-align "center-center"
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
| grid-align    | 9 positions: center-left, center-right, top-left, top-right, bottom-left, bottom-right, left-top, left-bottom, right-top, right-bottom |
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

