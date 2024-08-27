# rustyfications

Rusty notification daemon for Wayland.

---
![screenshot](assets/screenshot.png)

`Rust` | `Gtk4` | `gtk4-layer-shell`

---

## Running

Currently you need to run it manually. Be sure that no other notification daemons is running

```bash
cargo run --release
```

<details>
<summary><i style="display:inline-block">in case it throws `NameTaken` error</i></summary>

```bash
# check what other notification daemon is running
busctl --user list | grep org.freedesktop.Notifications
```

</details>
