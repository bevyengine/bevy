# Bevy FBX Loader

This crate provides basic support for importing FBX files into Bevy using the [`ufbx`](https://github.com/ufbx/ufbx-rust) library.

The loader converts meshes contained in an FBX scene into Bevy [`Mesh`] assets and groups them into a [`Scene`].
