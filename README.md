<!-- https://github.com/user-attachments/assets/37d267cb-1cb4-44b4-81a8-8ac02bb741cb -->



<!-- https://github.com/user-attachments/assets/e4b00c74-f4b1-4e45-9ff1-79f1b8bc6e13 -->



https://github.com/user-attachments/assets/46730461-267e-4930-8439-7dda360d7149




## Doc

Please refer to [https://way-edges.github.io/description]

## Installation

### Arch(aur)

- [way-edges-bin](https://aur.archlinux.org/packages/way-edges-bin)
- [way-edges-git](https://aur.archlinux.org/packages/way-edges-git)

### Manual

```shell
git clone https://github.com/way-edges/way-edges.git
cd way-edges && cargo build --release
```

## Configuration

Place `config.jsonc` under `~/.config/way-edges/`

Example config: https://github.com/ogios/dots/tree/master/way-edges

### Full doc

Doc: https://way-edges.github.io/basic_config

### Schema

Please refer to [config.jsonc](./config/config.jsonc) and [schema](./config/config.schema.json)

## Launch

1. Run daemon first(`way-edges daemon`).
2. Add group of widgets given group name specified in your configuration file(`way-edges add <group_name>`).
3. Some command require widget_name specified in order to operate.

```rust
Hidden widget on the screen edges

Usage: way-edges [OPTIONS] <COMMAND>

Commands:
  daemon     run daemon. There can only be one daemon at a time
  add        add group of widgets to applicatoin given group name
  rm         remove group of widgets to applicatoin given group name
  togglepin  toggle pin of a widget under certain group. format: <group_name>:<widget_name>
  quit       close daemon
  help       Print this message or the help of the given subcommand(s)

Options:
  -d, --mouse-debug  whether enable mouse click output, shoule be used width daemon command
  -h, --help         Print help
  -V, --version      Print version
```
