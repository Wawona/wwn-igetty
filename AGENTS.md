# wwn-igetty

L3′. Linux-shaped VTs after iland Mode B takes the display.

- Rust daemon (`igettyd`). C/ObjC only for Mach subscribe, libvterm cell
  bitfields, CoreText glyphs, and the Doorman `igetty` trampoline.
- Rust `wwn-igetty-core` is shared by macOS Classic and the in-process iOS
  TrollStore backend. iOS text sessions use Wawona zsh virtual PTYs.
- Do not put VT switching in `wwn-iland` or L4 Wawona.
- GUI VT is assigned (`WWN_IGETTY_GUI_VT`), not assumed to be 1.
- WindowServer replacement stays `wwn-iland` `iland-baremetal`.
- Doorman, Procursus host PTYs, and host APT are deferred to the Sileo provider.
  Never link them into the TrollStore static library.
