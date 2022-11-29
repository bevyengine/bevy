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

Graphics and audio need to be configured for them to work with WSL 2 backend.
Please see the ubuntu [WSL documentation](https://wiki.ubuntu.com/WSL) on how to set up graphics and audio.

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

Or if there are errors such as:

```txt
  --- stderr
  thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: "`\"pkg-config\" \"--libs\" \"--cflags\" \"libudev\"` did not exit successfully: exit status: 1\n--- stderr\nPackage libudev was not found in the pkg-config search path.\nPerhaps you should add the directory containing `libudev.pc'\nto the PKG_CONFIG_PATH environment variable\nNo package 'libudev' found\n"', /home/<user>/.cargo/registry/src/github.com-1ecc6299db9ec823/libudev-sys-0.1.4/build.rs:38:41
  stack backtrace:
     0: rust_begin_unwind
               at /rustc/9bb77da74dac4768489127d21e32db19b59ada5b/library/std/src/panicking.rs:517:5
     1: core::panicking::panic_fmt
               at /rustc/9bb77da74dac4768489127d21e32db19b59ada5b/library/core/src/panicking.rs:96:14
     2: core::result::unwrap_failed
               at /rustc/9bb77da74dac4768489127d21e32db19b59ada5b/library/core/src/result.rs:1617:5
     3: core::result::Result<T,E>::unwrap
               at /rustc/9bb77da74dac4768489127d21e32db19b59ada5b/library/core/src/result.rs:1299:23
     4: build_script_build::main
               at ./build.rs:38:5
     5: core::ops::function::FnOnce::call_once
               at /rustc/9bb77da74dac4768489127d21e32db19b59ada5b/library/core/src/ops/function.rs:227:5
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
warning: build failed, waiting for other jobs to finish...
error: build failed
```

Set the `PKG_CONFIG_PATH` env var to `/usr/lib/<target>/pkgconfig/`. For example on an x86_64 system:

```txt
export PKG_CONFIG_PATH="/usr/lib/x86_64-linux-gnu/pkgconfig/"
```

## Arch / Manjaro

```bash
sudo pacman -S libx11 pkgconf alsa-lib
```

Install `pipewire-alsa` or `pulseaudio-alsa` depending on the sound server you are using.

Note that for Intel GPUs, Vulkan drivers are not installed by default, you must also install
the `vulkan-intel` for bevy to work.

## Void

```bash
sudo xbps-install -S pkgconf alsa-lib-devel libX11-devel eudev-libudev-devel
```

## NixOS

Add a `shell.nix` file to the root of the project containing:

```nix
{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell rec {
  nativeBuildInputs = [
    pkg-config
    llvmPackages.bintools # To use lld linker
  ];
  buildInputs = [
    udev alsaLib vulkan-loader
    xlibsWrapper xorg.libXcursor xorg.libXrandr xorg.libXi # To use x11 feature
    libxkbcommon wayland # To use wayland feature
  ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
}
```

And enter it by just running `nix-shell`. You should be able compile Bevy programs using `cargo run` within this nix-shell. You can do this in one line with `nix-shell --run "cargo run"`.

Note that this template does not add Rust to the environment because there are many ways to do it. For example, to use stable Rust from nixpkgs you can add `cargo` to `nativeBuildInputs`.

## [OpenSUSE](https://www.opensuse.org/)

```bash
   sudo zypper install libudev-devel gcc-c++ alsa-lib-devel
```

## Gentoo

```bash
   sudo emerge --ask libX11 pkgconf alsa-lib
```

When using an AMD Radeon GPU, you may also need to emerge `amdgpu-pro-vulkan` to get Bevy to find the GPU.

## [Clear Linux OS](https://clearlinux.org/)

```bash
sudo swupd bundle-add devpkg-alsa-lib
sudo swupd bundle-add devpkg-libgudev
```
