# 💻 Way-Edges 0.5.0

- \* Migrate to [wayland-client](https://github.com/Smithay/client-toolkit) under smithay
- \* Replace `glib::Mainloop` with [calloop](https://github.com/Smithay/calloop)
- \* Remove refresh rate control
- \* Force scale=1 for ui under any output scaling, no blurry anymore
- \* Fix: Mouse position unsync when layer size changed.

- Remove input region calculation.
- Cpu usage dropped 80-90% during poping(14650hx+144hz)
- Memory usage dropped.
- Number of threads dropped.
- Replace notify-rs with inotify for file watching
- Replace libpulse-binding-glib with libpulse-binding-tokio
- Completely removed gtk4 & gtk4-layershell
- Use freedesktop-icons for icon fetching
- i forgor💀
