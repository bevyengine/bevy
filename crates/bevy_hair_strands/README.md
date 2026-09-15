# Bevy Hair Strands

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_hair_strands.svg)](https://crates.io/crates/bevy_hair_strands)
[![Downloads](https://img.shields.io/crates/d/bevy_hair_strands.svg)](https://crates.io/crates/bevy_hair_strands)
[![Docs](https://docs.rs/bevy_hair_strands/badge.svg)](https://docs.rs/bevy_hair_strands/latest/bevy_hair_strands/)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

Strand-based hair rendering for Bevy.

A [`HairStrands`] asset holds a set of polylines (one per strand). Each strand is
turned into a thin, view-facing ribbon on the GPU and shaded with a Kajiya-Kay /
Marschner-style anisotropic [`HairMaterial`], so it picks up the same lights,
shadows and fog as the rest of a `bevy_pbr` scene.
