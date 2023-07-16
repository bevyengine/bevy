#!/bin/bash
set -e

printf "\nBuilding generator...\n"

cmake -S . -B build
cmake --build build

printf "\nRunning generator...\n"

./build/mtg_generator "../data/suzanne_flat_tris.obj" "../data/suzanne_flat_tris.bin"
./build/mtg_generator "../data/suzanne_smooth_tris.obj" "../data/suzanne_smooth_tris.bin"
./build/mtg_generator "../data/cube.obj" "../data/cube.bin"
./build/mtg_generator "../data/suzanne_bad.obj" "../data/suzanne_bad.bin"
