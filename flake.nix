{
  description = "wwn-igetty: Linux-shaped virtual terminals + Doorman login on iland DRM after macOS WindowServer is replaced. L3'. Not the Mode B dylib (that is wwn-iland baremetal).";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    wwn-toolchain.url = "https://flakehub.com/f/Wawona/wwn-toolchain/*";
    wwn-toolchain.inputs.nixpkgs.follows = "nixpkgs";
    wwn-toolchain.inputs.rust-overlay.follows = "rust-overlay";
    wwn-iland.url = "https://flakehub.com/f/Wawona/wwn-iland/*";
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
          pkg = pkgs.callPackage ./default.nix {
            inherit iland;
            doorman = doorman.packages.${system}.doorman;
          };
        in
        {
          default = pkg;
          wwn-igetty = pkg;
          igettyd = pkg;
        }
      );
    };
}
