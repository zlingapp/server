{
  description = "Zling server, a blazing fast featureful chat app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain;

        nativeBuildInputs = with pkgs; [
          rustToolchain
          sqlx-cli
          # We need these to build mediasoup
          (python311.withPackages
            (ps: with ps; [
              meson
              ninja
              pip
            ]))
        ] ++ (if system == "aarch64-darwin" || system == "x86_64-darwin" then [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
          libiconv
          xcbuild
        ] else [
          # If someone else uses this flake (not apple silicon), it might not work.
          # Other deps might need to be added...
        ]);

        cargoToml = (pkgs.lib.importTOML ./Cargo.toml).package;
        name = cargoToml.name;
        version = cargoToml.version;
      in
      with pkgs;
      {

        devShells.default = mkShell {
          inherit nativeBuildInputs;
        };

        packages.default = rustPlatform.buildRustPackage {
          RUST_BACKTRACE = "1";
          inherit nativeBuildInputs name version;
          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;
        };
      }
    );
}
