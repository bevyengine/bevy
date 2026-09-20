# Bevy Hair Strands

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

Strand-based hair rendering for Bevy.

A [`HairStrands`] asset holds a set of polylines (one per strand). Each strand is
turned into a thin, view-facing ribbon on the GPU and shaded with a Kajiya-Kay
anisotropic [`HairMaterial`] (two shifted specular lobes, after Scheuermann), so
it picks up the same lights, shadows and fog as the rest of a `bevy_pbr` scene.

The ribbon mesh is packed to 20 bytes a vertex and lives in the render world
only. Far away, where a strand would be thinner than
`HairMaterial::min_pixel_width` pixels, only a fraction of the strands is drawn,
widened to keep the same coverage; each shadow map thins by its own resolution.
Culling bounds are padded from the material's widths unless a
`HairStrandsBoundsPadding` says otherwise.
