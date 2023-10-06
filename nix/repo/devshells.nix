{
  inputs,
  cell,
}: let
  inherit (inputs) nixpkgs std;
in
  nixpkgs.lib.mapAttrs (_: std.lib.dev.mkShell) {
    default = let
      nativeBuildInputs = with nixpkgs; [
        pkg-config
        wayland
        grim
      ];
      buildInputs = with nixpkgs; [
        wayland
        fontconfig
        libxkbcommon
        vulkan-loader
        libGL
      ];
    in {
      name = "Watershot";
      packages = with nixpkgs;
        [
          rustc
          clippy
          cargo
          cargo-watch
          cargo-edit
          cargo-tarpaulin
          cargo-nextest
          rustfmt
        ]
        ++ nativeBuildInputs
        ++ buildInputs;
      commands = [
        {package = nixpkgs.cargo;}
        {
          name = "workaround";
          # numtide devshells don't have the hook for pkg-config, but nix-shell does
          help = "devshell pkg-config workaround";
          command = let
            packages = with nixpkgs.lib; concatMapStringsSep " " getName buildInputs;
          in ''nix-shell -p pkg-config ${packages}'';
        }
        {
          name = "test-watch";
          help = "watch for changes and run nextest";
          command = ''cargo watch -x "nextest run"'';
        }
      ];
      nixago = with cell.configs; [
        treefmt
      ];
    };
  }
