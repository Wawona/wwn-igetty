#include <string.h>
#include <vterm.h>

int modeb_vterm_cell(VTermScreen *screen, int row, int col, uint32_t *cp,
                     int *reverse, int *bold, uint8_t *fg_rgb, uint8_t *bg_rgb) {
  VTermPos pos = {.row = row, .col = col};
  VTermScreenCell cell;
  memset(&cell, 0, sizeof(cell));
  if (!vterm_screen_get_cell(screen, pos, &cell))
    return -1;
  *cp = cell.chars[0];
  *reverse = cell.attrs.reverse ? 1 : 0;
  *bold = cell.attrs.bold ? 1 : 0;
  VTermColor fg = cell.fg;
  VTermColor bg = cell.bg;
  vterm_screen_convert_color_to_rgb(screen, &fg);
  vterm_screen_convert_color_to_rgb(screen, &bg);
  fg_rgb[0] = fg.rgb.red;
  fg_rgb[1] = fg.rgb.green;
  fg_rgb[2] = fg.rgb.blue;
  bg_rgb[0] = bg.rgb.red;
  bg_rgb[1] = bg.rgb.green;
  bg_rgb[2] = bg.rgb.blue;
  return 0;
}
