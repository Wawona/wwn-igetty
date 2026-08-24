# Wawona Mode B TTY. Login macOS path_helper drops getty PATH.
# Session wrappers must stay first so typed weston/niri use iland DRM.

_wwn_modeb_path() {
  [ -n "${WWN_MODEB_SESSION_BIN:-}" ] || return 0
  PATH="${WWN_MODEB_SESSION_BIN}${WWN_MODEB_BIN:+:${WWN_MODEB_BIN}}:${PATH}"
  export PATH
}

_wwn_modeb_source_home() {
  local f="$1"
  [ -n "$HOME" ] && [ -f "$HOME/$f" ] || return 0
  local _save="$ZDOTDIR"
  ZDOTDIR="$HOME"
  # shellcheck disable=SC1090
  . "$HOME/$f"
  ZDOTDIR="$_save"
}

if [ -f "$HOME/.zshenv" ]; then
  _wwn_modeb_source_home .zshenv
fi
