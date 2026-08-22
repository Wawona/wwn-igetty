# wwn-igetty

L3′. Linux-shaped VTs after iland Mode B takes the display.

- Rust daemon (`igettyd`). C/ObjC only for Mach subscribe, libvterm cell
  bitfields, CoreText glyphs, and the Doorman `igetty` trampoline.
- Do not put VT switching in `wwn-iland` or L4 Wawona.
- GUI VT is assigned (`WWN_IGETTY_GUI_VT`), not assumed to be 1.
- WindowServer replacement stays `wwn-iland` `iland-baremetal`.
