{pkgs ? import <nixpkgs> {}}: let
  inputs = import ./inputs.nix {inherit pkgs;};
in
  pkgs.mkShell (inputs
  // {
    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath inputs.buildInputs;
  })
