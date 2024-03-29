{
  inputs,
  cell,
}: let
  inherit (inputs) std nixpkgs;
in {
  treefmt = std.lib.cfg.treefmt {
    data.formatter = {
      nix = {
        command = "alejandra";
        includes = ["*.nix"];
      };
      prettier = {
        command = "prettier";
        options = ["--plugin" "prettier-plugin-toml" "--write"];
        includes = [
          "*.md"
          "*.mdx"
          "*.toml"
        ];
      };
      rustfmt = {
        command = "rustfmt";
        includes = [
          "*.rs"
        ];
      };
    };
    packages = with nixpkgs; [
      alejandra
      nodePackages.prettier
      nodePackages.prettier-plugin-toml
      rustfmt
    ];
    devshell.startup.prettier-plugin-toml = nixpkgs.lib.stringsWithDeps.noDepEntry ''
      export NODE_PATH=${nixpkgs.nodePackages.prettier-plugin-toml}/lib/node_modules:$NODE_PATH
    '';
  };
}
