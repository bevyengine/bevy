# Installing Linux dependencies

This page lists the required dependencies to build a Bevy project on your Linux machine.

If you don't see your distro present in the list, feel free to add the instructions in this document.

## [Ubuntu](https://ubuntu.com/)

```bash
sudo apt-get install g++ pkg-config libx11-dev libasound2-dev libudev-dev
```

if using Wayland, you will also need to install

```bash
sudo apt-get install libwayland-dev libxkbcommon-dev
```

Depending on your graphics card, you may have to install one of the following:
`vulkan-radeon`, `vulkan-intel`, or `mesa-vulkan-drivers`

Compiling with clang is also possible - replace the `g++` package with `clang`.

### Windows Subsystem for Linux (WSL 2)

WSL2's virtual GPU Driver only supports OpenGL, requiring you to cross-compile for Windows within WSL and then run the build on Windows. (See [#841](https://github.com/bevyengine/bevy/issues/841) and [microsoft/WSL#7790](https://github.com/microsoft/WSL/issues/7790))

#### Dependencies

First install the MinGW toolchain

```bash
sudo apt-get install mingw-w64
```

Add the rust target

```bash
rustup target add x86_64-pc-windows-gnu
```

#### Usage

Place the following into `.cargo/config.toml` for your project

```toml
[build]
target = "x86_64-pc-windows-gnu"
```

Alternatively to changing your config, when building or running specify the target manually

```bash
cargo build --target x86_64-pc-windows-gnu
```

```bash
cargo run --target x86_64-pc-windows-gnu
```

## [Fedora](https://getfedora.org/)

```bash
sudo dnf install gcc-c++ libX11-devel alsa-lib-devel systemd-devel
```

if using Wayland, you will also need to install

```bash
sudo dnf install wayland-devel libxkbcommon-devel
```

If there are errors with linking during the build process such as:

```bash
 = note: /usr/bin/ld: skipping incompatible /usr/lib/libasound.so when searching for -lasound
          /usr/bin/ld: skipping incompatible /usr/lib/libasound.so when searching for -lasound
          /usr/bin/ld: skipping incompatible /usr/lib/gcc/x86_64-redhat-linux/10/../../../libasound.so when searching for -lasound
          /usr/bin/ld: skipping incompatible /lib/libasound.so when searching for -lasound
          /usr/bin/ld: skipping incompatible /usr/lib/libasound.so when searching for -lasound
          /usr/bin/ld: cannot find -lasound
```

Add your arch to the end of the package to remove the linker error. For example:

```bash
sudo dnf install alsa-lib-devel.x86_64
```

## Arch / Manjaro

```bash
sudo pacman -S libx11 pkgconf alsa-lib
```

Install `pipewire-alsa` or `pulseaudio-alsa` depending on the sound server you are using.

## Void

```bash
sudo xbps-install -S pkgconf alsa-lib-devel libX11-devel eudev-libudev-devel
```

## NixOS

Add a `shell.nix` file to the root of the project containing:

```nix
# shell.nix

{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell {
  nativeBuildInputs = [
    pkgconfig
    clang lld # To use lld linker
  ];
  buildInputs = [
    udev alsaLib vulkan-loader
    x11 xorg.libXcursor xorg.libXrandr xorg.libXi # To use x11 feature
    libxkbcommon wayland # To use wayland feature
  ];
  shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
    udev alsaLib vulkan-loader
    libxkbcommon wayland # To use wayland feature
  ]}"'';
}
```

And enter it by just running `nix-shell`. You should be able compile bevy programs using `cargo` within this nix-shell.

Note that this template does not add Rust to the environment because there are many ways to do it, each with its pros and cons. For example, to use stable Rust from nixpkgs you can add `cargo` to `nativeBuildInputs`.

## [OpenSUSE](https://www.opensuse.org/)

```bash
   sudo zypper install libudev-devel gcc-c++ alsa-lib-devel
```

## Gentoo

```bash
   sudo emerge --ask libX11 pkgconf alsa-lib
```

## [Clear Linux OS](https://clearlinux.org/)

```bash
sudo swupd bundle-add devpkg-alsa-lib
sudo swupd bundle-add devpkg-libgudev
```
