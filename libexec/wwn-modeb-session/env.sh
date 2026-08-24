# Mode B TTY compositor env. Sourced by niri/weston wrappers.
# After Classic Take Over there is no host Wayland. iland DRM/KMS/GBM.
# Never export DYLD_INSERT_LIBRARIES here (Apple /bin/* is arm64e).
# Wrappers prefix insert on the compositor exec only.
unset WAYLAND_DISPLAY WAYLAND_SOCKET DISPLAY
export WWN_MODEB_TTY=1
export NIRI_BACKEND=tty
