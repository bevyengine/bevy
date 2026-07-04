{
  description = "Bevy development environment flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        bevyDeps =
          with pkgs;
          [
            pkg-config
            (rust-bin.stable.latest.default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })
          ]
          ++ lib.optionals stdenv.isLinux (
            with pkgs;
            [
              # Systembundles & Linkers
              lld
              clang

              # Audio
              alsa-lib

              # Graphics / Windowing
              vulkan-loader
              vulkan-tools
              wayland
              libx11
              libxcursor
              libxi
              libxrandr
              libxkbcommon

              # Input / Hardware
              libudev-zero
            ]
          );

        # Runtime libs for dlopen
        runtimeLibs = with pkgs; [
          vulkan-loader
          alsa-lib
          libx11
          libxcursor
          libxi
          libxrandr
          libxkbcommon
          wayland
          libudev-zero
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = bevyDeps;

          # Make Winit en Bevy find necessary libraries on Linux
          LD_LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isLinux (
            pkgs.lib.makeLibraryPath runtimeLibs
          );
        };
      }
    );
}
