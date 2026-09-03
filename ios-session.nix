{
  lib,
  pkgs,
  rustPlatform,
  simulator ? false,
  ...
}:

let
  cargoTarget = if simulator then "aarch64-apple-ios-sim" else "aarch64-apple-ios";
  rustToolchain = pkgs.rust-bin.stable.latest.default.override {
    targets = [ cargoTarget ];
  };
in
rustPlatform.buildRustPackage {
  pname = "wwn-igetty-ios";
  version = "0.1.0";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
  cargoBuildFlags = [
    "--package"
    "wwn-igetty-ios"
    "--target"
    cargoTarget
  ];
  doCheck = false;

  nativeBuildInputs = [ rustToolchain ];

  installPhase = ''
    runHook preInstall
    mkdir -p "$out/lib" "$out/include"
    cp "target/${cargoTarget}/release/libwwn_igetty_ios.a" "$out/lib/"
    cp crates/wwn-igetty-ios/include/wwn_igetty_ios.h "$out/include/"
    runHook postInstall
  '';

  meta = {
    description = "Wawona TrollStore logical session switcher";
    license = lib.licenses.mit;
    platforms = lib.platforms.darwin;
  };
}
