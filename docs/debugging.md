# Debugging

## Macro Debugging

* Print the final output of a macro using `cargo rustc --profile=check -- -Zunstable-options --pretty=expanded`
  * Alternatively you could install and use [cargo expand](https://github.com/dtolnay/cargo-expand) which adds syntax highlighting to the terminal output.
    * Additionally get pager by piping to `less` ( on Unix systems ): `cargo expand --color always | less -R`
* Print output during macro compilation using `eprintln!("hi");`

## WGPU Tracing

When a suspected wgpu error occurs, you should capture a wgpu trace so that Bevy and wgpu devs can debug using the [wgpu player tool](https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications#tracing-infrastructure).

To capture a wgpu trace:

1. Create a new folder in which to store your wgpu trace.
2. Pass the path to the folder you created for your wgpu trace to `bevy_render::RenderPlugin`, using the `render_creation` field.
   * If you're manually creating the renderer resources, pass the path to wgpu when creating the `RenderDevice` and `RenderQueue`.
   * Otherwise, pass the path to Bevy via the `trace_path` field in `bevy_render::settings::WgpuSettings`.
3. Add `wgpu = { version = "*", features = ["trace"]}` to your Cargo.toml.
   * `version = "*"` tells Rust that this can be *any* version of the wgpu crate, so it will not try to pull in a different version of wgpu than what is already pulled in by Bevy.
4. Compile and run your application, performing any in-app actions necessary to replicate the wgpu error.

Once you've captured a wgpu trace, zip up the folder and attach it to the relevant issue. New wgpu issues should generally be created [in the wgpu repository](https://github.com/gfx-rs/wgpu). Please include the wgpu revision in your bug reports. You can find the revision in the `Cargo.lock` file in your workspace.
