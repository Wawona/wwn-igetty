#import <CoreFoundation/CoreFoundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <CoreText/CoreText.h>
#include "modeb-tty-ctfont.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static CTFontRef g_font;
static int g_cw;
static int g_ch;
static CGColorSpaceRef g_cs;
static int g_yflip;
/* CoreDisplay presents the dumb IOSurface with origin at the bottom.
 * Cover bitmaps stay upright (selftest); blit row-reverses into the FB. */
static int g_present_rowflip = 1;

#define CACHE_N 4096
typedef struct {
  uint32_t cp;
  uint8_t *cover;
} cache_ent;
static cache_ent g_cache[CACHE_N];

int modeb_ctfont_ready(void) { return g_font != NULL && g_cw > 0 && g_ch > 0; }
int modeb_ctfont_rows_flipped(void) { return g_yflip; }

void modeb_ctfont_cell_size(int *w, int *h) {
  if (w)
    *w = g_cw;
  if (h)
    *h = g_ch;
}

static uint8_t *raster_into(uint32_t cp, int apply_flip) {
  size_t nbytes = (size_t)g_cw * (size_t)g_ch;
  uint8_t *cover = (uint8_t *)calloc(1, nbytes);
  if (!cover)
    return NULL;
  size_t rowb = (size_t)g_cw * 4;
  uint8_t *argb = (uint8_t *)calloc(1, rowb * (size_t)g_ch);
  if (!argb) {
    free(cover);
    return NULL;
  }

  CGContextRef ctx = CGBitmapContextCreate(
      argb, (size_t)g_cw, (size_t)g_ch, 8, rowb, g_cs,
      kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little);
  if (!ctx) {
    free(argb);
    free(cover);
    return NULL;
  }
  CGContextSetRGBFillColor(ctx, 0, 0, 0, 1);
  CGContextFillRect(ctx, CGRectMake(0, 0, g_cw, g_ch));
  /* Native Quartz: origin bottom-left. Do not flip the CTM (that inverted
   * glyphs when the text matrix stayed identity). */
  CGContextSetTextMatrix(ctx, CGAffineTransformIdentity);

  UTF32Char u32 = (UTF32Char)cp;
  if (u32 == 0)
    u32 = (UTF32Char)' ';
  UniChar units[2];
  UniCharCount n = 0;
  if (u32 <= 0xFFFF) {
    units[0] = (UniChar)u32;
    n = 1;
  } else {
    UTF32Char t = u32 - 0x10000;
    units[0] = (UniChar)(0xD800 + (t >> 10));
    units[1] = (UniChar)(0xDC00 + (t & 0x3FF));
    n = 2;
  }
  CGGlyph glyphs[2] = {0, 0};
  CTFontGetGlyphsForCharacters(g_font, units, glyphs, (CFIndex)n);
  CGContextSetRGBFillColor(ctx, 1, 1, 1, 1);
  CGPoint p = CGPointMake(0, CTFontGetDescent(g_font));
  if (glyphs[0])
    CTFontDrawGlyphs(g_font, glyphs, &p, 1, ctx);
  CGContextRelease(ctx);

  for (int yb = 0; yb < g_ch; yb++) {
    int src = apply_flip ? (g_ch - 1 - yb) : yb;
    uint8_t *row = argb + (size_t)src * rowb;
    for (int x = 0; x < g_cw; x++) {
      uint8_t r = row[x * 4 + 2];
      uint8_t g = row[x * 4 + 1];
      uint8_t b = row[x * 4 + 0];
      cover[yb * g_cw + x] = (uint8_t)((r + g + b) / 3);
    }
  }
  free(argb);
  return cover;
}

static int cover_upright_T(const uint8_t *cover) {
  if (!cover || g_ch < 6)
    return 0;
  int band = g_ch / 3;
  if (band < 1)
    band = 1;
  unsigned top = 0, bot = 0;
  for (int y = 0; y < band; y++) {
    for (int x = 0; x < g_cw; x++)
      top += cover[y * g_cw + x];
  }
  for (int y = g_ch - band; y < g_ch; y++) {
    for (int x = 0; x < g_cw; x++)
      bot += cover[y * g_cw + x];
  }
  /* Upright 'T': the bar lives in the top third. */
  return top > bot;
}

int modeb_ctfont_assert_upright(void) {
  uint8_t *c = raster_into((uint32_t)'T', g_yflip);
  if (!c)
    return -1;
  int ok = cover_upright_T(c);
  free(c);
  return ok ? 0 : -1;
}

static uint8_t *raster_cp(uint32_t cp) {
  uint32_t slot = cp % CACHE_N;
  if (g_cache[slot].cp == cp && g_cache[slot].cover)
    return g_cache[slot].cover;
  uint8_t *cover = raster_into(cp, g_yflip);
  if (!cover)
    return NULL;
  free(g_cache[slot].cover);
  g_cache[slot].cp = cp;
  g_cache[slot].cover = cover;
  return cover;
}

static CTFontRef font_from_url_regular(const char *path, float pt_size) {
  CFURLRef url = CFURLCreateFromFileSystemRepresentation(
      kCFAllocatorDefault, (const UInt8 *)path, (CFIndex)strlen(path), false);
  if (!url)
    return NULL;
  CTFontManagerRegisterFontsForURL(url, kCTFontManagerScopeProcess, NULL);
  CFArrayRef descs = CTFontManagerCreateFontDescriptorsFromURL(url);
  CFRelease(url);
  if (!descs)
    return NULL;
  CTFontRef chosen = NULL;
  CFIndex n = CFArrayGetCount(descs);
  for (CFIndex i = 0; i < n; i++) {
    CTFontDescriptorRef d = CFArrayGetValueAtIndex(descs, i);
    CTFontRef f = CTFontCreateWithFontDescriptor(d, pt_size, NULL);
    if (!f)
      continue;
    CFStringRef ps = CTFontCopyPostScriptName(f);
    char buf[128] = {0};
    if (ps) {
      CFStringGetCString(ps, buf, sizeof(buf), kCFStringEncodingUTF8);
      CFRelease(ps);
    }
    int is_reg = (strstr(buf, "Regular") != NULL);
    if (is_reg) {
      if (chosen)
        CFRelease(chosen);
      chosen = f;
      break;
    }
    CFRelease(f);
  }
  if (!chosen && n > 0) {
    chosen = CTFontCreateWithFontDescriptor(
        CFArrayGetValueAtIndex(descs, n > 1 ? 1 : 0), pt_size, NULL);
  }
  CFRelease(descs);
  return chosen;
}

static int font_is_helvetica_fallback(CTFontRef f) {
  if (!f)
    return 1;
  CFStringRef fam = CTFontCopyFamilyName(f);
  char buf[128] = {0};
  if (fam) {
    CFStringGetCString(fam, buf, sizeof(buf), kCFStringEncodingUTF8);
    CFRelease(fam);
  }
  return strstr(buf, "Helvetica") != NULL || strstr(buf, "UIFont") != NULL;
}

int modeb_ctfont_init(const char *ttf_path, float pt_size) {
  (void)ttf_path;
  if (pt_size < 8.f)
    pt_size = 12.f;

  const char *files[] = {
      "/System/Library/Fonts/SFNSMono.ttf",
      "/System/Library/Fonts/Menlo.ttc",
      "/System/Library/Fonts/Monaco.ttf",
  };
  g_font = NULL;
  for (size_t i = 0; !g_font && i < sizeof(files) / sizeof(files[0]); i++)
    g_font = font_from_url_regular(files[i], pt_size);
  if (g_font && font_is_helvetica_fallback(g_font)) {
    CFRelease(g_font);
    g_font = NULL;
  }
  if (!g_font)
    g_font = CTFontCreateWithName(CFSTR("Menlo"), pt_size, NULL);
  if (!g_font || font_is_helvetica_fallback(g_font)) {
    fprintf(stderr, "[igettyd] CoreText mono font create failed\n");
    if (g_font) {
      CFRelease(g_font);
      g_font = NULL;
    }
    return -1;
  }

  CGFloat ascent = CTFontGetAscent(g_font);
  CGFloat descent = CTFontGetDescent(g_font);
  CGFloat leading = CTFontGetLeading(g_font);
  g_ch = (int)ceil(ascent + descent + leading + 1.0);
  UniChar m = (UniChar)'M';
  CGGlyph glyph = 0;
  CTFontGetGlyphsForCharacters(g_font, &m, &glyph, 1);
  CGSize adv = {0, 0};
  if (glyph)
    CTFontGetAdvancesForGlyphs(g_font, kCTFontOrientationHorizontal, &glyph,
                               &adv, 1);
  g_cw = (int)ceil(adv.width > 1.0 ? adv.width : pt_size * 0.6);
  if (g_cw < 7)
    g_cw = 7;
  if (g_ch < 12)
    g_ch = 12;
  g_cs = CGColorSpaceCreateDeviceRGB();
  memset(g_cache, 0, sizeof(g_cache));

  /* Probe 'T': pick the row order where the bar is on top. */
  g_yflip = 0;
  uint8_t *probe = raster_into((uint32_t)'T', 0);
  if (probe && !cover_upright_T(probe))
    g_yflip = 1;
  free(probe);
  if (modeb_ctfont_assert_upright() != 0) {
    fprintf(stderr, "[igettyd] FATAL: SF Mono 'T' still inverted\n");
    return -1;
  }

  CFStringRef full = CTFontCopyFullName(g_font);
  char name[128] = {0};
  if (full) {
    CFStringGetCString(full, name, sizeof(name), kCFStringEncodingUTF8);
    CFRelease(full);
  }
  fprintf(stderr,
          "[igettyd] font '%s' cell %dx%d pt=%.1f yflip=%d present_rowflip=%d "
          "(SF Mono)\n",
          name, g_cw, g_ch, (double)pt_size, g_yflip, g_present_rowflip);
  return 0;
}

void modeb_ctfont_blit(uint32_t *fb, uint32_t pitch_bytes, int fb_w, int fb_h,
                       int px, int py, uint32_t cp, uint32_t fg_bgra,
                       uint32_t bg_bgra) {
  if (!fb || g_cw <= 0)
    return;
  uint8_t *cover = raster_cp(cp);
  uint8_t fr = (uint8_t)(fg_bgra & 0xff);
  uint8_t fg = (uint8_t)((fg_bgra >> 8) & 0xff);
  uint8_t fb_ = (uint8_t)((fg_bgra >> 16) & 0xff);
  uint8_t br = (uint8_t)(bg_bgra & 0xff);
  uint8_t bg = (uint8_t)((bg_bgra >> 8) & 0xff);
  uint8_t bb = (uint8_t)((bg_bgra >> 16) & 0xff);
  int pitch = (int)(pitch_bytes / 4);
  for (int y = 0; y < g_ch; y++) {
    int Y = py + y;
    if (Y < 0 || Y >= fb_h)
      continue;
    for (int x = 0; x < g_cw; x++) {
      int X = px + x;
      if (X < 0 || X >= fb_w)
        continue;
      int cy = g_present_rowflip ? (g_ch - 1 - y) : y;
      uint8_t a = cover ? cover[cy * g_cw + x] : 0;
      uint8_t r = (uint8_t)((br * (255 - a) + fr * a) / 255);
      uint8_t g = (uint8_t)((bg * (255 - a) + fg * a) / 255);
      uint8_t b = (uint8_t)((bb * (255 - a) + fb_ * a) / 255);
      fb[Y * pitch + X] =
          0xFF000000u | ((uint32_t)b << 16) | ((uint32_t)g << 8) | r;
    }
  }
}
