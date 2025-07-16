<https://github.com/user-attachments/assets/ca4fd799-a174-4072-b9e7-929ce9bbc1fe>

## 🔍 Doc

Please refer to [doc](doc) directory.

> [!WARNING]
> **master branch always refer to the latest but may not be released configurations, if you are using a released binary, please match the corresponding commit with the tag you are on**.
> 
> You can always find that in the [tag](https://github.com/way-edges/way-edges/tags) page:
> - enter the tag page
> - click on the commit hash link under the corresponding tag
> - click **Browse files** on the right 

## 📥 Installation

### Arch(aur)

- [way-edges-bin](https://aur.archlinux.org/packages/way-edges-bin)
- [way-edges-git](https://aur.archlinux.org/packages/way-edges-git) (recommended)

### Manual

```shell
git clone https://github.com/way-edges/way-edges.git
cd way-edges && cargo build --release
```

## ⚙️ Example config

my own config: <https://github.com/ogios/dots/tree/master/way-edges>  
i'm using `-git` version of the package in aur, the configurations may differ from `-bin`

## 🚀 Launch

1. Run daemon first(`way-edges`).
2. Some command require widget namespace specified in order to operate.

## 💛 Thanks

Special thanks for:

- [JakeStanger/system-tray](https://github.com/JakeStanger/system-tray).
- [Rayzeq/tryfol](https://github.com/Rayzeq/tryfol)
- [elkowar/eww](https://github.com/elkowar/eww)
- [YaLTeR/niri](https://github.com/YaLTeR/niri)
- [danieldg/rwaybar](https://github.com/danieldg/rwaybar)
