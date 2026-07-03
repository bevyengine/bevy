{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    alsa-lib
    libudev-zero
    libx11
    libxcursor
    libxi
    libxrandr
    libxkbcommon
    vulkan-loader
    wayland
  ];

  env = {
    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
      with pkgs;
      [
        libx11
        libxcursor
        libxi
        libxkbcommon
        vulkan-loader
        wayland
      ]
    );
  };
}
