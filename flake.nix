{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        # setup pkgs
        pkgs = import nixpkgs {
          inherit system;
          overlays = [fenix.overlays.default];
          config = {
            android_sdk.accept_license = true;
            allowUnfree = true;
          };
        };
        # the entire rust toolchain with required targets
        rustToolchain = with fenix.packages.${system};
          combine [
            (stable.withComponents [
              "rustc"
              "cargo"
              "rustfmt"
              "clippy"
              "rust-src"
            ])

            targets.wasm32-unknown-unknown.stable.rust-std
            targets.aarch64-linux-android.stable.rust-std
            targets.x86_64-pc-windows-gnu.stable.rust-std
          ];
      in {
        devShells.default = pkgs.mkShell rec {
          # build dependencies
          nativeBuildInputs = with pkgs; [
            # the entire rust toolchain
            rustToolchain
            # tool for cross compiling
            cargo-apk
            # xbuild

            pkg-config
            openssl

            # Common cargo tools we often use
            cargo-deny
            cargo-expand
            cargo-binutils

            # cmake for openxr
            cmake
          ];

          # runtime dependencies
          buildInputs =
            [
              pkgs.zstd
              pkgs.libxml2
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux (with pkgs; [
              # bevy dependencies
              udev
              alsa-lib
              # vulkan
              vulkan-loader
              vulkan-headers
              vulkan-tools
              vulkan-validation-layers
              # x11
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXrandr
              # wayland
              libxkbcommon
              wayland
              # xr
              openxr-loader
              libGL
            ])
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Cocoa
              # # This is missing on mac m1 nix, for some reason.
              # # see https://stackoverflow.com/a/69732679
              pkgs.libiconv
            ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          # this is most likely not needed. for some reason shadows flicker without it.
          AMD_VULKAN_ICD = "RADV";
        };
        # This only formats the nix files.
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
