{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];

        pkgs = import nixpkgs { inherit system overlays; };

        toolchain = pkgs.rust-bin.stable.latest.default.override { extensions = [ "rust-src" ]; };

        libraries = with pkgs; [];

        cargo-extensions = with pkgs; [
          cargo-audit
          cargo-edit
          cargo-watch
        ];
      in
      {
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs =
              with pkgs;
              [
                pkg-config
                toolchain
                gcc
              ]
              ++ libraries
              ++ cargo-extensions;

            env = {
              RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
            };
          };
      }
    );
}
