{
  description = "NuNuShell development environment - Lean";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      devshell,
    }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      eachSystem =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f {
            inherit system;
            pkgs = nixpkgs.legacyPackages.${system};
            fenix-pkg = fenix.packages.${system};
          }
        );
    in
    {
      devShells = eachSystem (
        {
          pkgs,
          system,
          fenix-pkg,
        }:
        let
          rust-nightly = fenix-pkg.complete.withComponents [
            "cargo"
            "clippy"
            "rust-src"
            "rustc"
            "rustfmt"
          ];
        in
        {
          default = (devshell.legacyPackages.${system}.mkShell) {
            name = "NuNuShell";

            packages = [
              rust-nightly
              pkgs.rust-analyzer
              pkgs.cargo-nextest
              pkgs.git
              pkgs.nixfmt-rfc-style
              pkgs.openssl
              pkgs.pkg-config
            ];

            commands = [
              {
                name = "test";
                help = "Run tests with nextest";
                command = "cargo nextest run";
              }
              {
                name = "lint";
                help = "Run clippy";
                command = "cargo clippy";
              }
            ];
          };
        }
      );

      formatter = eachSystem ({ pkgs, ... }: pkgs.nixfmt-rfc-style);
    };
}
