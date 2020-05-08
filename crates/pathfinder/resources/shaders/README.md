This directory contains postprocessed versions of the shaders in the top-level
`shaders/` directory, for convenience. Don't modify the shaders here; instead
modify the corresponding shaders in `shaders/` and rerun `make` in that
directory.

You will need `glslangValidator` and `spirv-cross` installed to execute the
Makefile. On macOS, you can get these with `brew install glslang spirv-cross`.
