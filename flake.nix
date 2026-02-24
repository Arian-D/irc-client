{
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
    nixpak = {
      url = "github:nixpak/nixpak";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { nixpkgs, nixpak, fenix, flake-utils, ... }:

    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
        mkNixPak = nixpak.lib.nixpak {
            inherit (pkgs) lib; 
            inherit pkgs;
            };
      in {
        devShells.default = import ./devshell.nix {
          inherit pkgs mkNixPak;
          fenix = fenix.packages.${system};
        };
      });
}
