{
  description = "A simple wayland native screenshot tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    std = {
      url = "github:divnix/std";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    std,
    ...
  } @ inputs:
    std.growOn {
      inherit inputs;
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
