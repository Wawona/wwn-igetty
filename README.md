# wwn-igetty (L3′)

Linux-shaped **virtual terminals** for Wawona Desktop Replacement (Mode B)
after WindowServer is gone. Not the Mode B `.dylib`. That dylib and own-display
KMS live in **wwn-iland** `iland-baremetal`.

```text
iland-baremetal dylib  →  take over display (framebufferd / inputd)
wwn-igetty (this repo) →  VT switcher + Doorman getty on text VTs
assigned GUI VT        →  weston / niri (DRM backend) or kmscube / gbm-es2 / vkcube-kms
```

Weston and niri are **not** DRM-only. Machines Start still nests them
(`--backend=wayland` / `NIRI_BACKEND=nested`) when Display Backend is Wayland.
After Classic Take Over there is no host Wayland, so the GUI VT uses each
compositor's own DRM/KMS/GBM backend (`--backend=drm` / `NIRI_BACKEND=tty`).
Typing `niri` or `weston` on a Doorman text VT is a **first-class** path: same
iland DRM/KMS/GBM session as the assigned GUI VT. Login env sets
`NIRI_BACKEND=tty`, clears nested `WAYLAND_DISPLAY`, sets `ZDOTDIR` so login
zsh keeps session wrappers first (ahead of `~/.local/bin`). Those wrappers
prefix `DYLD_INSERT_LIBRARIES` from `WWN_MODEB_INSERT` on the compositor exec
only, as the login user. Never sudo. Never a real `/dev/dri` node. iland
userspace DRM is what `open("/dev/dri/...")` becomes. Do not export insert in
the login shell. Do not nest. Mode A Machines Start is unchanged.

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
| `WWN_MODEB_GBM_ES2` | gbm-es2-demo for Ctrl+Option+F8 |
| `WWN_MODEB_VKCUBE` | vkcube-kms for Ctrl+Option+F9 |

`libwayland-mac.dylib` publishes `/tmp/libwayland-support/modeb-drm-client.pid`
while a DRM client (typed `weston`/`niri`, GUI VT compositor, F7-F9 overlay) is
running. igettyd skips text-VT pageflips while that pid is alive so console dumb
buffers do not reclaim the panel from a compositor.

Text VTs (every number 1-6 except the GUI VT) run `igetty` (Doorman). Switch
with Ctrl+Option+F1-F6 (Option is Alt). Overlay KMS clients: F7 kmscube, F8
gbm-es2-demo, F9 vkcube-kms. Arrows, Page Up/Page Down, Home, and End send
CSI on text VTs (MacBook: hold Fn with arrows). Ctrl+Option+Backspace restores Aqua.
Fn+Ctrl+Option+Backspace does the same (MacBook Fn remaps Backspace to Delete).

Text drawing matches Linux `fbcon` on a mapped scanout buffer: damage rects
(`fbcon_putcs`), pixel copy on scroll (`fbcon_bmove`), fillrect for blanks
(`fbcon_clear`), and a 200ms block cursor that inverts one cell
(`fbcon_cursor` / `fbcon_flashcursor`). igettyd ping-pongs two dumb BOs and
pageflips. CoreDisplay ignores `presentSurface` of the same IOSurface, which
is why F7 kmscube then F1 showed a live TTY and in-place flips did not.
vterm callbacks use a VT index, not `&Vt`. `App` is boxed after
`vt_init`, so a `&Vt` from the stack `App` would dangle and Doorman
login text would never dirty the live VTs (F7 then F1 looked fine
because a VT switch forces `full_redraw`).

Auth: [Doorman](https://github.com/Wawona/doorman) as a library. Do not call
`doorman_open_session()` (that forks/setsid off the PTY).

DAG: toolchain + iland (DRM present) + doorman. Never Wawona, never weston as
a flake input. Cited in `Wawona/docs/wwn-repo-dag.md`.

## iOS TrollStore logical sessions

`wwn-igetty-core` owns the platform-neutral session state machine.
`wwn-igetty-ios` is an in-process static library for
`com.aspauldingcode.Wawona.ModeB`. It switches the Machines greeter, native
clients, compositors, JIT VM/container sessions, and up to eight Wawona zsh
PTY slots over one IOMFB output. Text input focus is selected with
`wwn_ios_terminal_set_master`; it does not use `/dev/tty`, `fork`, a host
login shell, or Doorman.

The `wwn-igetty-jailbreak` crate reserves the same provider contract for a
later Sileo implementation. It is intentionally not linked into TrollStore or
App Store products. That later provider may add Doorman authentication,
Procursus PTYs, and host APT.
