# Prepend session wrappers so `weston`/`niri` are iland DRM, not nested
# CLI wrappers from ~/.local/bin (login zsh path_helper).
if [ -n "${WWN_MODEB_SESSION_BIN-}" ]; then
  PATH="${WWN_MODEB_SESSION_BIN}${WWN_MODEB_BIN:+:${WWN_MODEB_BIN}}:${PATH}"
  export PATH
fi
