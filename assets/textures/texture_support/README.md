# Texture Format Sample Image Generation

## Required Software

- [**toktx**](https://github.khronos.org/KTX-Software/ktxtools/toktx.html)
- [**basisu**](https://github.com/BinomialLLC/basis_universal)

## Instructions

The files in this directory are generated from [png-srgb-rgb.png](./png-srgb-rgb.png) using the following commands:

### Misc LDR Formats

```sh
pip install pillow imageio qoi numpy
python ./scripts/export-ldr-formats.py
```

### EXR / HDR

```sh
pip install imageio numpy OpenEXR
python ./scripts/export-hdr-formats.py
```

### KTX2

```sh
rm -f ./*.ktx2

# KTX2: Uncompressed
toktx --t2 --assign_oetf srgb srgb8.ktx2 png-srgb-rgb.png

# KTX2: Uncompressed HDR
# https://github.khronos.org/KTX-Software/ktxtools/ktx_create.html
# https://registry.khronos.org/vulkan/specs/latest/man/html/VkFormat.html
ktx create --format R32_SFLOAT exr-hdr.exr ktx2-hdr-r32.ktx2
ktx create --format R32G32B32_SFLOAT exr-hdr.exr ktx2-hdr-rgb32.ktx2
ktx create --format R32G32B32A32_SFLOAT exr-hdr.exr ktx2-hdr-rgba32.ktx2

# KTX2: ASTC sRGB w/Mips
toktx --t2 --genmipmap --assign_oetf srgb --encode astc --astc_blk_d 4x4 ktx2-astc-4x4-srgb-mips.ktx2 png-srgb-rgb.png

# KTX2: ASTC sRGB
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 4x4 ktx2-astc-4x4-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 5x4 ktx2-astc-5x4-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 5x5 ktx2-astc-5x5-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 6x5 ktx2-astc-6x5-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 6x6 ktx2-astc-6x6-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 8x5 ktx2-astc-8x5-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 8x6 ktx2-astc-8x6-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 10x5 ktx2-astc-10x5-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 10x6 ktx2-astc-10x6-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 8x8 ktx2-astc-8x8-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 10x8 ktx2-astc-10x8-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 10x10 ktx2-astc-10x10-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 12x10 ktx2-astc-12x10-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode astc --astc_blk_d 12x12 ktx2-astc-12x12-srgb.ktx2 png-srgb-rgb.png

# KTX2: ASTC Linear
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 4x4 ktx2-astc-4x4-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 5x4 ktx2-astc-5x4-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 5x5 ktx2-astc-5x5-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 6x5 ktx2-astc-6x5-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 6x6 ktx2-astc-6x6-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 8x5 ktx2-astc-8x5-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 8x6 ktx2-astc-8x6-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 10x5 ktx2-astc-10x5-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 10x6 ktx2-astc-10x6-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 8x8 ktx2-astc-8x8-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 10x8 ktx2-astc-10x8-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 10x10 ktx2-astc-10x10-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 12x10 ktx2-astc-12x10-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --astc_blk_d 12x12 ktx2-astc-12x12-linear.ktx2 png-srgb-rgb.png

# KTX2: ASTC w/Zstd Supercompression
toktx --t2 --assign_oetf srgb --encode astc --zcmp 10 --astc_blk_d 4x4 ktx2-zstd-astc-4x4-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode astc --zcmp 10 --astc_blk_d 4x4 ktx2-zstd-astc-4x4-linear.ktx2 png-srgb-rgb.png

# KTX2: UASTC
toktx --t2 --assign_oetf srgb --encode uastc ktx2-uastc-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --encode uastc --uastc_rdo_l 1.0 ktx2-uastc-srgb-rdo.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode uastc ktx2-uastc-linear.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode uastc --uastc_rdo_l 1.0 ktx2-uastc-linear-rdo.ktx2 png-srgb-rgb.png

# KTX2: UASTC w/Zstd Supercompression
toktx --t2 --assign_oetf srgb --encode uastc --zcmp 10 --uastc_rdo_l 1.0 ktx2-zstd-uastc-srgb-rdo.ktx2 png-srgb-rgb.png

# KTX2: ETC1S/BasisLZ
toktx --t2 --assign_oetf srgb --encode etc1s ktx2-etc1s-srgb.ktx2 png-srgb-rgb.png
toktx --t2 --assign_oetf srgb --convert_oetf linear --encode etc1s ktx2-etc1s-linear.ktx2 png-srgb-rgb.png

# KTX2: Multi-layer with Mipmaps
toktx --t2 --layers 6 --genmipmap --assign_oetf srgb --encode astc --astc_blk_d 4x4 ktx2-astc-4x4-srgb-multilayer-mips.ktx2 \
    png-layer-0.png \
    png-layer-1.png \
    png-layer-2.png \
    png-layer-3.png \
    png-layer-4.png \
    png-layer-5.png

# KTX2: Cubemap
# toktx --t2 --cubemap --assign_oetf srgb --encode astc --astc_blk_d 4x4 ktx2-astc-4x4-srgb-cubemap.ktx2 \
#     png-layer-0.png \
#     png-layer-1.png \
#     png-layer-2.png \
#     png-layer-3.png \
#     png-layer-4.png \
#     png-layer-5.png

# KTX2: Cubemap with Mipmaps
# toktx --t2 --cubemap --genmipmap --assign_oetf srgb --encode astc --astc_blk_d 4x4 ktx2-astc-4x4-srgb-cubemap-mips.ktx2 \
#     png-layer-0.png \
#     png-layer-1.png \
#     png-layer-2.png \
#     png-layer-3.png \
#     png-layer-4.png \
#     png-layer-5.png
```

### Basis

```sh
rm -f ./*.basis

# Basis: ETC1S
basisu -basis -etc1s png-srgb-rgb.png -output_file ./basis-etc1s-srgb.basis
basisu -basis -etc1s -linear png-srgb-rgb.png -output_file ./basis-etc1s-linear.basis

# Basis: ETC1S Video / Cubemap / 2darray
# basisu -basis -etc1s -tex_type video ./png-layer-0.png png-layer-1.png png-layer-2.png png-layer-3.png png-layer-4.png png-layer-5.png -output_file ./basis-etc1s-srgb-video.basis
# basisu -basis -etc1s -tex_type cubemap png-layer-0.png png-layer-1.png png-layer-2.png png-layer-3.png png-layer-4.png png-layer-5.png -output_file ./basis-etc1s-srgb-cubemap.basis

# Basis: ETC1S Video / Cubemap / 2darray (with mips)
# basisu -basis -mipmap -etc1s -tex_type video ./png-layer-0.png png-layer-1.png png-layer-2.png png-layer-3.png png-layer-4.png png-layer-5.png -output_file ./basis-etc1s-srgb-video-mips.basis
# basisu -basis -mipmap -etc1s -tex_type cubemap png-layer-0.png png-layer-1.png png-layer-2.png png-layer-3.png png-layer-4.png png-layer-5.png -output_file ./basis-etc1s-srgb-cubemap-mips.basis
# basisu -basis -mipmap -etc1s -tex_type 2darray png-layer-0.png png-layer-1.png png-layer-2.png png-layer-3.png png-layer-4.png png-layer-5.png -output_file ./basis-etc1s-srgb-multilayer-mips.basis

# Basis: UASTC 4x4 LDR
basisu -basis -uastc png-srgb-rgb.png -output_file ./basis-uastc-4x4-srgb.basis
basisu -basis -uastc -uastc_rdo_l 2.0 png-srgb-rgb.png -output_file ./basis-uastc-4x4-rdo-srgb.basis
basisu -basis -uastc -mipmap png-srgb-rgb.png -output_file ./basis-uastc-4x4-srgb-mips.basis

# Basis: UASTC 4x4 LDR w/Mips
basisu -basis -mipmap -uastc -tex_type 2darray png-layer-0.png png-layer-1.png png-layer-2.png png-layer-3.png png-layer-4.png png-layer-5.png -output_file ./basis-uastc-4x4-srgb-multilayer-mips.basis

# Basis: KTX2 HDR
basisu -hdr_4x4 exr-hdr.exr -output_file ./ktx2-astc-4x4-hdr.ktx2
basisu -hdr_6x6 exr-hdr.exr -output_file ./ktx2-astc-6x6-hdr.ktx2
basisu -hdr_6x6i exr-hdr.exr -output_file ./ktx2-astc-6x6i-hdr.ktx2

# Basis: Basis HDR
basisu -basis -hdr_4x4 exr-hdr.exr -output_file ./basis-astc-4x4-hdr.basis
basisu -basis -hdr_6x6 exr-hdr.exr -output_file ./basis-astc-6x6-hdr.basis
basisu -basis -hdr_6x6i exr-hdr.exr -output_file ./basis-astc-6x6i-hdr.basis
```

## For Benches

```sh
ktx create --generate-mipmap --format R32G32B32A32_SFLOAT exr-hdr.exr ../../../benches/benches/bevy_image/assets/ktx2-rgba32-mips.ktx2

toktx --t2 -resize 555x555 --genmipmap --assign_oetf srgb --encode uastc ../../../benches/benches/bevy_image/assets/ktx2-uastc-srgb-mips.ktx2 png-srgb-rgb.png

toktx --t2 --zcmp 10 -resize 555x555 --genmipmap --assign_oetf srgb --encode uastc ../../../benches/benches/bevy_image/assets/ktx2-zstd-uastc-srgb-mips.ktx2 png-srgb-rgb.png
```