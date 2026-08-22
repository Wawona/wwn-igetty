#include "modeb-tty-ctfont.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
  if (modeb_ctfont_init(NULL, 12.f) != 0) {
    fprintf(stderr, "init failed\n");
    return 1;
  }
  int w = 0, h = 0;
  modeb_ctfont_cell_size(&w, &h);
  if (modeb_ctfont_assert_upright() != 0) {
    fprintf(stderr, "FAIL upright T yflip=%d cell=%dx%d\n",
            modeb_ctfont_rows_flipped(), w, h);
    return 2;
  }
  printf("PASS SF Mono 12pt cell=%dx%d yflip=%d upright T\n", w, h,
         modeb_ctfont_rows_flipped());
  return 0;
}
