# Debugging

## Macro Debugging

* Print the final output of a macro using `cargo rustc --profile=check -- -Zunstable-options --pretty=expanded`
  * Alternatively you could install and use [cargo expand](https://github.com/dtolnay/cargo-expand) which adds syntax highlighting to the terminal output.
    * Additionally get pager by piping to `less` ( on Unix systems ): `cargo expand --color always | less -R`
* Print output during macro compilation using `eprintln!("hi");`

## WGPU Tracing

When a suspected wgpu error occurs, you should capture a wgpu trace so that Bevy and wgpu devs can debug using the [wgpu player tool](https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications#tracing-infrastructure).

To capture a wgpu trace:

1. Create a new folder in which to store your wgpu trace
2. Pass the folder path to `bevy_render::RenderPlugin`, using the `render_creation` field.
   * If you're manually creating the renderer resources, pass the path to wgpu when creating the `RenderDevice` and `RenderQueue`.
   * Otherwise, pass the path to Bevy via the `trace_path` field in `bevy_render::settings::WgpuSettings`.
3. Enable wgpu's trace feature and run your application
   1. Add `wgpu = "*"` to your Cargo.toml (this is effectively saying it can be *any* version of the wgpu crate, so it will not try to pull in a different version of wgpu than what is already pulled in by Bevy).
   2. Execute `cargo run --features wgpu/trace`.
4. Zip up the folder and attach it to the relevant issue. New wgpu issues should generally be created [in the wgpu repository](https://github.com/gfx-rs/wgpu). Please include the wgpu revision in your bug reports. You can find the revision in the `Cargo.lock` file in your workspace.
