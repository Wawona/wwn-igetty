# Prefix iland Mode B insert on the compositor exec. The login user types
# `niri` / `weston` (never sudo). open("/dev/dri/...") is iland via Dobby.
# Do not export DYLD_INSERT_LIBRARIES in the parent shell.
wwn_modeb_exec() {
  if [ -z "${WWN_MODEB_INSERT-}" ] && [ -r /tmp/libwayland-support/modeb-insert ]; then
    WWN_MODEB_INSERT=$(sed -n '1p' /tmp/libwayland-support/modeb-insert)
    export WWN_MODEB_INSERT
  fi
  if [ -z "${WWN_MODEB_INSERT-}" ]; then
    echo "modeb: WWN_MODEB_INSERT unset. Type niri after Classic Take Over" >&2
    echo "so the session has iland userspace DRM insert. Never sudo niri." >&2
    echo "Never open a real DRM node." >&2
    exit 1
  fi
  export DYLD_INSERT_LIBRARIES="$WWN_MODEB_INSERT"
  log="${WWN_MODEB_CLIENT_LOG:-/tmp/wawona-modeb-client.log}"
  exec >>"$log" 2>&1
  exec "$@"
}
