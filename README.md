# wwn-igetty (L3′)

Linux-shaped **virtual terminals** for Wawona Desktop Replacement (Mode B)
after WindowServer is gone. Not the Mode B `.dylib`. That dylib and own-display
KMS live in **wwn-iland** `iland-baremetal`.

```text
iland-baremetal dylib  →  take over display (framebufferd / inputd)
wwn-igetty (this repo) →  VT switcher + Doorman getty on text VTs
assigned GUI VT        →  weston / niri / kmscube (Desktop Machine)
```

The GUI session is **assigned a VT**. It is not hardcoded as VT1. Linux often
puts GDM on `/dev/tty1`, but a display manager can pick another number.
`WWN_IGETTY_GUI_VT` (default 1 when a GUI command is set) is that assignment.

Text VTs show a `tty01`..`tty06` label in base16 `base0B` (terminal green,
solarized-dark `#859900`).

| Env | Meaning |
|-----|---------|
| `WWN_IGETTY_GUI_VT` | 1-6. VT that runs the DRM compositor / DE |
| `WWN_IGETTY_GUI_CMD` | argv0 (weston, niri, kmscube, …) |
| `WWN_IGETTY_GUI_ARGS` | remaining argv, separated by U+001F |
| `WWN_IGETTY_GETTY` | Doorman login helper (`igetty`) |
| `WWN_MODEB_KMSCUBE` | kmscube for Ctrl+Option+F7 |

Text VTs (every number 1-6 except the GUI VT) run `igetty` (Doorman). Switch
with Ctrl+Option+F1-F6 (Option is Alt). Ctrl+Option+F7 starts kmscube on the
same DRM path. Ctrl+Option+Backspace restores Aqua.

Auth: [Doorman](https://github.com/Wawona/doorman) as a library. Do not call
`doorman_open_session()` (that forks/setsid off the PTY).

DAG: toolchain + iland (DRM present) + doorman. Never Wawona, never weston as
a flake input. Cited in `Wawona/docs/wwn-repo-dag.md`.
