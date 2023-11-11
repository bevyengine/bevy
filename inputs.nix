{pkgs}: with pkgs; {
  nativeBuildInputs = [
    cargo
    pkg-config
  ];
  buildInputs = [
    udev
    alsa-lib
    vulkan-loader
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr # To use the x11 feature
    libxkbcommon
    wayland # To use the wayland feature
  ];
}
