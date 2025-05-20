# Install

## Arch(aur)

- [way-edges-bin](https://aur.archlinux.org/packages/way-edges-bin)
- (recommended)[way-edges-git](https://aur.archlinux.org/packages/way-edges-git)

## Build instruction

`tokio_unstable` cfg is required.

But it's already specified in `cargo/config.toml` and `neoconf.json`, so you can just:

```shell
cargo build --release
```
