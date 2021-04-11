# Installing Linux dependencies

This page lists the required dependencies to build a Bevy project on your Linux machine.

If you don't see your distro present in the list, feel free to add the instructions in this document.

## Ubuntu 20.04

```bash
sudo apt-get install pkg-config libx11-dev libasound2-dev libudev-dev
```

Depending on your graphics card, you may have to install one of the following:
`vulkan-radeon`, `vulkan-intel`, or `mesa-vulkan-drivers`

If you want to enable fast compiles

```bash
sudo apt-get install clang
```

### Windows Subsystem for Linux (WSL 2)

Graphics and audio need to be configured for them to work with WSL 2 backend.
Please see the ubuntu [WSL documentation](https://wiki.ubuntu.com/WSL) on how to set up graphics and audio.

## Fedora 33

```bash
sudo dnf install gcc-c++ libX11-devel alsa-lib-devel systemd-devel
```

## Arch / Manjaro

```bash
sudo pacman -S libx11 pkgconf alsa-lib
```

## Solus

```bash
sudo eopkg install pkg-config libx11-devel g++ alsa-lib-devel
```

## Void

```bash
sudo xbps-install -S pkgconf alsa-lib-devel libX11-devel eudev-libudev-devel
```

## NixOS

Add a `build.rs` file to your project containing:

```rust
# build.rs

fn main() {
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=vulkan");
    }
}
```

These packages provide the dependencies required to run a bevy project. They can be installed globally or via nix-shell.
Based on your global configuration it also might be necessary to allow unfree packages:

```bash
export NIXPKGS_ALLOW_UNFREE=1 # needed for lutris
nix-shell -p cargo pkgconfig udev lutris alsaLib x11 xorg.libXcursor xorg.libXrandr xorg.libXi vulkan-tools vulkan-headers vulkan-loader vulkan-validation-layers
```

Alternatively, you can define `shell.nix` containing:

```nix
# shell.nix

{ pkgs ? import <nixpkgs> { } }:
with pkgs;
mkShell {
  buildInputs = [
    cargo
    pkgconfig udev alsaLib lutris
    x11 xorg.libXcursor xorg.libXrandr xorg.libXi
    vulkan-tools vulkan-headers vulkan-loader vulkan-validation-layers
  ];
}
```

And enter it by just running `nix-shell`.

You should be able compile bevy programms using `cargo` within this nix-shell.
