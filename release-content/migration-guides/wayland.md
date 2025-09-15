---
title: Enable Wayland by default
pull_requests: [19232]
---

Wayland has now been added to the default features of the `bevy` crate.

```text
  called `Result::unwrap()` on an `Err` value:
  pkg-config exited with status code 1
  > PKG_CONFIG_ALLOW_SYSTEM_LIBS=1 PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1 pkg-config --libs --cflags wayland-client

  The system library `wayland-client` required by crate `wayland-sys` was not found.
  The file `wayland-client.pc` needs to be installed and the PKG_CONFIG_PATH environment variable must contain its parent directory.
  The PKG_CONFIG_PATH environment variable is not set.

  HINT: if you have installed the library, try setting PKG_CONFIG_PATH to the directory containing `wayland-client.pc`.
```

If you've encountered an error message similar to the one above, this means that you will want to make the `wayland-client` library available to your build system, or disable default features, in order to successfully build Bevy on Linux.

On Ubuntu, or other Debian-based distributions, install the `libwayland-dev` package:

```sh
sudo apt install libwayland-dev
```

On Arch Linux:

```sh
sudo pacman -S wayland
```

On Nix, add the `wayland` package to your `buildInputs`:

```nix
buildInputs = [ pkgs.wayland ];
```
