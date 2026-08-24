# After /etc/zprofile path_helper.
if [ -f "$HOME/.zprofile" ]; then
  _wwn_modeb_source_home .zprofile
fi
_wwn_modeb_path
