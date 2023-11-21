
{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, fenix, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system: 
    let
      toolchain = fenix.packages.${system}.stable.toolchain;
      pkgs = nixpkgs.legacyPackages.${system};
      nodejs = pkgs.nodejs-18_x;
      yarn = pkgs.yarn.override { nodejs = pkgs.nodejs-18_x; };
      npm = pkgs.nodePackages.npm.override { nodejs = pkgs.nodejs-18_x; };
    in
    {
      devShells.default = pkgs.mkShell {
        nativeBuildInputs =
            [
              pkgs.cargo-nextest
              nodejs
              pkgs.cargo-cross
              pkgs.sqlx-cli
              pkgs.mailhog # Required for testing SMTP
              yarn
              npm
              pkgs.python310
              pkgs.python310Packages.gyp
              fenix.packages.${system}.stable.toolchain
            ];
      };

      nixosModules = rec {
        tiberius = import ./service.nix self;
        default = tiberius;
      };
      
      packages.default =

        (pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
          withComponents = with pkgs; [
            nixpkgs.cargo-nextest
          ];
        }).buildRustPackage {
          pname = "tiberius";
          version = "0.1.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          cargoLock.outputHashes = {
            "comrak-0.15.0" = "sha256-JMGMXfftu82PBnsi4vdfSxQ47DjhxiNG82abQ4OmefI=";
            "sqlx-adapter-0.4.2" = "sha256-tBPGuBvmcd6QhtuA68L9JdhmuSfzg9Gt1AdfSrxf1RE=";
          };

          nativeBuildInputs = [
              nodejs npm yarn
              pkgs.python310
              pkgs.python310Packages.gyp
          ];

          # disable networked tests
          checkNoDefaultFeatures = true;
          checkFeatures = [ ];

          useNextest = true;
        };
    });
}