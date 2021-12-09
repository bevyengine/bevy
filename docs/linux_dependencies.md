# Installing Linux dependencies

This page lists the required dependencies to build a Bevy project on your Linux machine.

If you don't see your distro present in the list, feel free to add the instructions in this document.

## Ubuntu 20.04

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

Graphics and audio need to be configured for them to work with WSL 2 backend.
Please see the ubuntu [WSL documentation](https://wiki.ubuntu.com/WSL) on how to set up graphics and audio.

## Fedora

```bash
sudo dnf install gcc-c++ libX11-devel alsa-lib-devel systemd-devel
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

### Fast compilation

According to the Bevy getting started guide (for v0.5), you can enable fast compilation by add a Cargo config file and by adding `lld` and `clang`. As long as you add `clang` and `lld` to your environment, it should mostly work, but you'll still need to modify the Cargo config file so that it doesn't point to `/usr/bin/clang` anymore.

Working off the above files, let's make the necessary changes.

For `.cargo/config.toml`, change the path to the linker from `/usr/bin/clang` to `clang`:

``` diff
  [target.x86_64-unknown-linux-gnu]
- linker = "/usr/bin/clang"
+ linker = "clang"
  rustflags = ["-Clink-arg=-fuse-ld=lld", "-Zshare-generics=y"]
```

In `shell.nix`, add `lld` and `clang`:

``` diff
  buildInputs = [
    cargo
    pkgconfig udev alsaLib lutris
    x11 xorg.libXcursor xorg.libXrandr xorg.libXi
    vulkan-tools vulkan-headers vulkan-loader vulkan-validation-layers
+   clang lld
  ];
```

### Building apps and using the GPU

If you run into issues with building basic apps or activating the GPU ('thread 'main' panicked at 'Unable to find a GPU!'), then you may need to update your environment's `LD_LIBRARY_PATH`. To solve issues relating to missing `libudev.so.1` files, `alsa` drivers, and being unable to find a GPU, try updating the environment variable in your `shell.nix` by creating a `shellHook`:

``` diff
  { pkgs ? import <nixpkgs> { } }:
  with pkgs;
  mkShell {
+   shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
+     pkgs.alsaLib
+     pkgs.udev
+     pkgs.vulkan-loader
+   ]}"'';
    buildInputs = [
```

## Opensuse Tumbleweed

```bash
   sudo zypper install libudev-devel gcc-c++
```
