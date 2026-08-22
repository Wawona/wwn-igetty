# wwn-igetty: Linux-shaped VTs + Doorman login on iland DRM (macOS Mode B).
# Daemon is Rust. Mach / vterm bitfields / CoreText stay thin C/ObjC glue.
{
  lib,
  pkgs,
  iland,
  doorman,
  ...
}:

let
  libvterm = pkgs.libvterm-neovim;
in
pkgs.stdenv.mkDerivation {
  pname = "wwn-igetty";
  version = "0.1.0";

  src = ./.;

  __noChroot = true;
  dontConfigure = true;

  nativeBuildInputs = [
    pkgs.rustc
    pkgs.clang
  ];
  buildInputs = [
    libvterm
    doorman
  ];

  buildPhase = ''
    runHook preBuild
    unset DEVELOPER_DIR
    MACOS_SDK=$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || true)
    if [ ! -d "$MACOS_SDK" ]; then
      MACOS_SDK="/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk"
    fi
    export SDKROOT="$MACOS_SDK"
    CLANG="${pkgs.clang}/bin/clang"
    INCLUDES="-I${iland}/include -I${libvterm}/include -I${doorman}/include"
    CFLAGS="-isysroot $SDKROOT -mmacosx-version-min=12.0 -O2 -std=c11 $INCLUDES -I."
    OBJCFLAGS="-isysroot $SDKROOT -mmacosx-version-min=12.0 -O2 $INCLUDES -I."
    FRAMEWORKS="-framework IOSurface -framework Foundation -framework CoreFoundation -framework IOKit -framework CoreGraphics -framework ApplicationServices -framework QuartzCore -framework Metal -framework Cocoa -framework CoreText"
    AUTH_FRAMEWORKS="-framework Foundation -framework OpenDirectory -framework Security"
    echo "CC igetty (Doorman)"
    "$CLANG" $CFLAGS modeb-getty.c \
      ${doorman}/lib/libdoorman.a \
      $AUTH_FRAMEWORKS -lpam -lobjc \
      -o igetty
    echo "CC CoreText + Mach + vterm shims"
    "$CLANG" $OBJCFLAGS -c modeb-tty-ctfont.m -o modeb-tty-ctfont.o
    "$CLANG" $CFLAGS -c modeb-tty-input.c -o modeb-tty-input.o
    "$CLANG" $CFLAGS -c modeb-tty-vterm.c -o modeb-tty-vterm.o
    echo "CC font selftest"
    "$CLANG" $CFLAGS modeb-tty-font-selftest.c modeb-tty-ctfont.m $FRAMEWORKS -o modeb-tty-font-selftest
    ./modeb-tty-font-selftest
    echo "rustc igettyd"
    ${pkgs.rustc}/bin/rustc --edition 2021 -C opt-level=2 \
      -C debuginfo=0 \
      src/main.rs -o igettyd \
      -C link-arg=-isysroot -C link-arg="$SDKROOT" \
      -C link-arg=-mmacosx-version-min=12.0 \
      -C link-arg=modeb-tty-ctfont.o \
      -C link-arg=modeb-tty-input.o \
      -C link-arg=modeb-tty-vterm.o \
      -L native=${iland}/lib \
      -L native=${libvterm}/lib \
      -l iland_userland \
      -l vterm \
      -C link-arg=-Wl,-rpath,${libvterm}/lib \
      -C link-arg=-lobjc \
      -C link-arg=-framework -C link-arg=IOSurface \
      -C link-arg=-framework -C link-arg=Foundation \
      -C link-arg=-framework -C link-arg=CoreFoundation \
      -C link-arg=-framework -C link-arg=IOKit \
      -C link-arg=-framework -C link-arg=CoreGraphics \
      -C link-arg=-framework -C link-arg=ApplicationServices \
      -C link-arg=-framework -C link-arg=QuartzCore \
      -C link-arg=-framework -C link-arg=Metal \
      -C link-arg=-framework -C link-arg=Cocoa \
      -C link-arg=-framework -C link-arg=CoreText
    runHook postBuild
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp igettyd igetty $out/bin/
    ln -sf igettyd $out/bin/modeb-ttyd
    ln -sf igetty $out/bin/modeb-getty
  '';

  meta = with lib; {
    description = "Wawona igetty: Linux-shaped VT switcher + Doorman login on iland DRM";
    license = licenses.mit;
    platforms = platforms.darwin;
  };
}
