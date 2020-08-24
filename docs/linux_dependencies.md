# Installing Linux dependencies

This page lists the required dependencies to build a Bevy project on your Linux machine.

If you don't see your distro present in the list, feel free to add the instructions in this document.

## Ubuntu 20.04
```bash
sudo apt-get install libx11-dev libasound2-dev
```

## Fedora 32
```bash
sudo dnf install gcc-c++ libX11-devel alsa-lib-devel
```

## Arch / Manjaro
```bash
sudo pacman -S libx11 pkgconf alsa-lib
```
