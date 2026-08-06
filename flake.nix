{
  description = "flake for rust";

  outputs =
    {
      nixpkgs,
      ...
    }:
    let
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed (system: f nixpkgs.legacyPackages.${system});
    in
    {
      # packages = forAllSystems (pkgs: {
      #   default = pkgs.rustPlatform.buildRustPackage {
      #     pname = "something";
      #     version = "1.0.0";

      #     src = ./.;
      #     cargoLock.lockFile = ./Cargo.lock;
      #   };
      # });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            stdenv.cc.cc.lib
            rust-analyzer
            cargo
            rustc
            alsa-lib
            pkg-config
          ];

          
          LD_LIBRARY_PATH = "${nixpkgs.lib.makeLibraryPath [pkgs.alsa-lib]}";
        };

      });

    };
}
