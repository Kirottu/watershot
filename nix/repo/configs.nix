{
  inputs,
  cell,
}: let
  inherit (inputs) std nixpkgs;
  inherit (std.lib) dev cfg;
in {
  treefmt = dev.mkNixago cfg.treefmt {
    data.formatter = {
      nix = {
        command = "alejandra";
        includes = ["*.nix"];
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
      rustfmt
    ];
  };
}
