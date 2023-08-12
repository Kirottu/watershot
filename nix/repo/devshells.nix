{
  inputs,
  cell,
}: let
  inherit (inputs) nixpkgs std;
in
  nixpkgs.lib.mapAttrs (_: std.lib.dev.mkShell) {
    default = {
      name = "Watershot";
      packages = with nixpkgs; [
        # Rust tooling
        rustc
        rustfmt
        clippy
        cargo
        cargo-watch
        cargo-edit
        cargo-tarpaulin
        cargo-nextest

        # Dependencies
        fontconfig
        pkgconfig
        libxkbcommon
        grim
      ];
      nixago = with cell.configs; [
        treefmt
      ];
    };
  }
