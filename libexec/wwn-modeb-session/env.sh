# Mode B TTY compositor env. Sourced by niri/weston wrappers.
# After Classic Take Over there is no host Wayland. iland DRM/KMS/GBM.
unset WAYLAND_DISPLAY WAYLAND_SOCKET DISPLAY
export WWN_MODEB_TTY=1
export NIRI_BACKEND=tty
if [ -n "${WWN_MODEB_INSERT-}" ]; then
  export DYLD_INSERT_LIBRARIES="$WWN_MODEB_INSERT"
fi
