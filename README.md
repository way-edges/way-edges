<https://github.com/user-attachments/assets/366e506a-a476-4c4a-be17-678a8f1cd759>

## 🫧 Project board

<https://github.com/orgs/way-edges/projects/1/views/1>

## 🔍 Doc

Please refer to [doc](doc)

## 📥 Installation

### Arch(aur)

- [way-edges-bin](https://aur.archlinux.org/packages/way-edges-bin)
- [way-edges-git](https://aur.archlinux.org/packages/way-edges-git)

### Manual

```shell
git clone https://github.com/way-edges/way-edges.git
cd way-edges && cargo build --release
```

## ⚙️ Configuration

Doc: [doc](doc/config)

Example config: <https://github.com/ogios/dots/tree/master/way-edges>

## 🚀 Launch

1. Run daemon first(`way-edges`).
2. Add group of widgets given group name specified in your configuration file(`way-edges add <group_name>`).
3. Some command require widget_name specified in order to operate.

## 💛 Thanks

Special thanks for:

- [JakeStanger/system-tray](https://github.com/JakeStanger/system-tray). I forked one for zbus5.0 version: <https://github.com/ogios/system-tray-zbus5>
- [Rayzeq/tryfol](https://github.com/Rayzeq/tryfol)
- [elkowar/eww](https://github.com/elkowar/eww)
- [YaLTeR/niri](https://github.com/YaLTeR/niri)
- [danieldg/rwaybar](https://github.com/danieldg/rwaybar)
