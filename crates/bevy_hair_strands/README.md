# Bevy Hair Strands

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

Strand-based hair rendering for Bevy.

A [`HairStrands`] asset holds a set of polylines (one per strand), each with an
optional colour and width of its own. Each strand is turned into a thin,
view-facing ribbon on the GPU and shaded with a Kajiya-Kay anisotropic
[`HairMaterial`] (two shifted specular lobes, after Scheuermann), so it picks up
the same lights, shadows and fog as the rest of a `bevy_pbr` scene, and the same
indirect light: the ambient colour, environment maps, irradiance volumes and
screen-space ambient occlusion. Points deep in the hair mass are darkened by an
occlusion baked from the strand density around them.

The ribbon mesh is packed to 20 bytes a vertex (24 with per-strand colour) and
lives in the render world only. A strand thinner than a pixel is drawn a pixel
wide at its true coverage, through alpha-to-coverage under multisampling and an
ordered dither without. Far away, where a strand would be thinner than
`HairMaterial::min_pixel_width` pixels, only a fraction of the strands is drawn,
widened to keep the same coverage; each shadow map thins by its own resolution.
Culling bounds are padded from the material's and strands' widths unless a
`HairStrandsBoundsPadding` says otherwise.
