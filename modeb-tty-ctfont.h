/* CoreText rasterizer for Mode B TTY. Face is macOS SF Mono 12pt. */
#ifndef MODEB_TTY_CTFONT_H_
#define MODEB_TTY_CTFONT_H_

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int modeb_ctfont_init(const char *ttf_path, float pt_size);
void modeb_ctfont_cell_size(int *w, int *h);
int modeb_ctfont_ready(void);
/* 1 if glyph bitmaps were row-reversed so cover[0] is the top of the letter. */
int modeb_ctfont_rows_flipped(void);
/* Draw one cell into a BGRA framebuffer (pitch in bytes). */
void modeb_ctfont_blit(uint32_t *fb, uint32_t pitch_bytes, int fb_w, int fb_h,
                       int px, int py, uint32_t cp, uint32_t fg_bgra,
                       uint32_t bg_bgra);
/* Returns 0 if 'T' has more ink in the top third than the bottom third. */
int modeb_ctfont_assert_upright(void);

#ifdef __cplusplus
}
#endif
#endif
