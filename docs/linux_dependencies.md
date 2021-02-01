# Installing Linux dependencies

This page lists the required dependencies to build a Bevy project on your Linux machine.

If you don't see your distro present in the list, feel free to add the instructions in this document.

## Ubuntu 20.04

```bash
sudo apt-get install pkg-config libx11-dev libasound2-dev libudev-dev
```
If you want to Enable Fast Compiles
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

Add a `build.rs` file to your project with the following:

```rust
fn main() {
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=vulkan");
    }
}
```

The following packages are known to provide the dependencies required to run a bevy project. They can be installed globally or via nix-shell.

`nix-shell -p pkgconfig x11 xorg.libXcursor xorg.libXrandr xorg.libXi vulkan-tools lutris vulkan-headers vulkan-loader vulkan-validation-layers alsaLib`

Alternatively, you can copy the following code block and create a file called `shell.nix`. You can now enter nix-shell just by running `nix-shell`.

```nix
# shell.nix

{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = [
    pkgs.alsaLib
    pkgs.lutris
    pkgs.pkgconfig
    pkgs.vulkan-headers
    pkgs.vulkan-loader
    pkgs.vulkan-tools
    pkgs.vulkan-validation-layers
    pkgs.x11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXi
    pkgs.xorg.libXrandr
  ];
}

```

At this point, projects should successfully compile but fail on execution. This is due to `glslang_validator` which, unfortunately, needs to have it's binary patched to link correctly. This is a known issue, and there are plans to remove this dependency.

1. `find target -type f -name glslang_validator` in order to find glslang_validator in `target/debug/build/bevy-glsl-to-spirv-<hash>/out/glslang_validator`. The directory containing glslang_validator will be referenced again, so save it for later: `export OUT_DIR="$(dirname $(find target -type f -name glslang_validator))"`.
2. Running `ldd $OUT_DIR/glslang_validator` may show `libstdc++.so.6` is not found. If all dependencies are found, then bevy should work. If not, install (globally or in nix-shell) any of the results found by `nix-locate -w libstdc++.so.6`. For example purposes, consider `nixos.gcc-unwrapped`. In theory, any of the ones in `find -L /nix/store -type f -name libstdc++.so.6` will work.
3. `patchelf --set-interpreter "$(cat $NIX_CC/nix-support/dynamic-linker)" --set-rpath /nix/store/784rh7jrfhagbkydjfrv68h9x3g4gqmk-gcc-8.3.0-lib/lib $OUT_DIR/glslang_validator`
4. Bevy should now be working correctly!
