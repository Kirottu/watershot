{
  inputs,
  cell,
}: let
  inherit (inputs) self std nixpkgs;
  inherit (nixpkgs) rustPlatform;
  cargo = builtins.fromTOML (builtins.readFile (self + "/Cargo.toml"));
in {
  default = rustPlatform.buildRustPackage {
    pname = cargo.package.name;
    version = cargo.package.version;

    src = std.incl self [
      "Cargo.toml"
      "Cargo.lock"
      "src"
    ];

    cargoLock.lockFile = self + "/Cargo.lock";

    nativeBuildInputs = with nixpkgs; [
      pkg-config
    ];

    buildInputs = with nixpkgs; [
      fontconfig
      libxkbcommon
      wayland
    ];
  };
}
