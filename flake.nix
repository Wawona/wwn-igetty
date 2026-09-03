{
  description = "wwn-igetty: Rust logical sessions for Wawona Desktop. macOS Classic uses iland DRM and Doorman; iOS TrollStore uses in-process Wawona PTYs.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    wwn-toolchain.url = "https://flakehub.com/f/Wawona/wwn-toolchain/*";
    wwn-toolchain.inputs.nixpkgs.follows = "nixpkgs";
    wwn-toolchain.inputs.rust-overlay.follows = "rust-overlay";
    # IOMFB and Mode B coordinate shims are consumed from development.
    # Cited: Wawona/docs/wwn-repo-dag.md.
    wwn-iland.url = "github:Wawona/wwn-iland/development";
    wwn-iland.inputs.nixpkgs.follows = "nixpkgs";
    wwn-iland.inputs.rust-overlay.follows = "rust-overlay";
    wwn-iland.inputs.wwn-toolchain.follows = "wwn-toolchain";
    doorman.url = "github:Wawona/doorman";
  };

  outputs =
    { self, nixpkgs, rust-overlay, wwn-toolchain, wwn-iland, doorman, ... }:
    let
      darwinSystems = [ "aarch64-darwin" ];
      forAll = nixpkgs.lib.genAttrs darwinSystems;
      inherit (wwn-toolchain.lib) baseRegistry mkToolchains;
    in
    {
      packages = forAll (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
            config = {
              allowUnfree = true;
              allowUnsupportedSystem = true;
            };
          };
          tc = mkToolchains {
            inherit pkgs;
            registry = baseRegistry // wwn-iland.registryFragment;
          };
          iland = tc.buildForMacOS "iland" { };
          modebCoordSrc = wwn-iland + "/dependencies/libs/iland/upstream/shims";
          pkg = pkgs.callPackage ./default.nix {
            inherit iland modebCoordSrc;
            doorman = doorman.packages.${system}.doorman;
          };
          iosSession = pkgs.callPackage ./ios-session.nix { inherit pkgs; };
        in
        {
          default = pkg;
          wwn-igetty = pkg;
          igettyd = pkg;
          wwn-igetty-ios = iosSession;
        }
      );
    };
}
