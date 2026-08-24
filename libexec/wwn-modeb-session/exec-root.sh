# Re-exec niri/weston as root via the Mode B helper. wayland-mac abort()s
# unless euid is 0. Do not export DYLD_INSERT_LIBRARIES in the login shell.
# The helper inserts only on the compositor exec, and only while
# WindowServer is down. It must not restore Aqua or touch watchdogd.
wwn_modeb_exec() {
  if [ "$(id -u)" -eq 0 ]; then
    exec "$@"
  fi
  helper="${WWN_MODEB_HELPER:-/Library/Application Support/Wawona/run-modeb.sh}"
  if [ -x "$helper" ]; then
    exec /usr/bin/sudo -n "$helper" --exec-compositor -- "$@"
  fi
  echo "modeb: niri/weston on a text VT needs Classic Take Over and $helper" >&2
  echo "wayland-mac must run as root. Do not export DYLD_INSERT_LIBRARIES." >&2
  exit 1
}
