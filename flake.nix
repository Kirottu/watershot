{
  description = "A simple wayland native screenshot tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    devshell.url = "github:numtide/devshell";
    nixago.url = "github:nix-community/nixago";
    std = {
      url = "github:divnix/std";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.devshell.follows = "devshell";
      inputs.nixago.follows = "nixago";
    };
  };

  outputs = {
    self,
    std,
    ...
  } @ inputs:
    std.growOn {
      inherit inputs;
      systems = ["x86_64-linux" "aarch64-linux"];
      cellsFrom = self + "/nix";
      cellBlocks = with std.blockTypes; [
        (installables "packages")
        (devshells "devshells")
        (nixago "configs")
      ];
    }
    {
      packages = std.harvest self ["watershot" "packages"];
      devShells = std.harvest self ["repo" "devshells"];
    };
}
