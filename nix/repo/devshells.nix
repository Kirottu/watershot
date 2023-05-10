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
        rustc
        cargo
        rustfmt
        clippy

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
