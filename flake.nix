{
  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix";
    flake-utils.follows = "cargo2nix/flake-utils";
    nixpkgs.follows = "cargo2nix/nixpkgs";
  };

  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ cargo2nix.overlays.default ];
        };
        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustVersion = "1.61.0";
          packageFun = import ./Cargo.nix;
          packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "evdev-sys";
              overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                  pkgs.libevdev
                ];
              };
            })
          ];
        };
      in
      rec {
        packages = {
          evdev-share = (rustPkgs.workspace.evdev-share { }).bin;
          default = packages.evdev-share;
        };
      }
    );
}
