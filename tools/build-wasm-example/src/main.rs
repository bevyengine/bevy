use std::{fs::File, io::Write};

use clap::{Parser, ValueEnum};
use xshell::{cmd, Shell};

#[derive(Debug, Copy, Clone, ValueEnum)]
enum WebApi {
    Webgl2,
    Webgpu,
}

#[derive(Parser, Debug)]
struct Args {
    /// Examples to build
    examples: Vec<String>,

    #[arg(short, long)]
    /// Run tests
    test: bool,

    #[arg(short, long)]
    /// Run on the given browsers. By default, chromium, firefox, webkit
    browsers: Vec<String>,

    #[arg(short, long)]
    /// Stop after this number of frames
    frames: Option<usize>,

    #[arg(value_enum, short, long, default_value_t = WebApi::Webgl2)]
    /// Browser API to use for rendering
    api: WebApi,

    #[arg(short, long)]
    /// Optimize the wasm file for size with wasm-opt
    optimize_size: bool,
}

fn main() {
    let cli = Args::parse();

    assert!(!cli.examples.is_empty(), "must have at least one example");

    let mut default_features = true;
    let mut features = vec![];
    if let Some(frames) = cli.frames {
        let mut file = File::create("ci_testing_config.ron").unwrap();
        file.write_fmt(format_args!("(exit_after: Some({frames}))"))
            .unwrap();
        features.push("bevy_ci_testing");
    }

    match cli.api {
        WebApi::Webgl2 => (),
        WebApi::Webgpu => {
            features.push("animation");
            features.push("bevy_asset");
            features.push("bevy_audio");
            features.push("bevy_gilrs");
            features.push("bevy_scene");
            features.push("bevy_winit");
            features.push("bevy_core_pipeline");
            features.push("bevy_pbr");
            features.push("bevy_gltf");
            features.push("bevy_render");
            features.push("bevy_sprite");
            features.push("bevy_text");
            features.push("bevy_ui");
            features.push("png");
            features.push("hdr");
            features.push("ktx2");
            features.push("zstd");
            features.push("vorbis");
            features.push("x11");
            features.push("bevy_gizmos");
            features.push("android_shared_stdcxx");
            features.push("tonemapping_luts");
            features.push("default_font");
            default_features = false;
        }
    }

    for example in cli.examples {
        let sh = Shell::new().unwrap();
        let features_string = features.join(",");
        let mut parameters = vec![];
        if !default_features {
            parameters.push("--no-default-features");
        }
        if !features.is_empty() {
            parameters.push("--features");
            parameters.push(&features_string);
        }
        let mut cmd = cmd!(
            sh,
            "cargo build {parameters...} --profile release --target wasm32-unknown-unknown --example {example}"
        );
        if matches!(cli.api, WebApi::Webgpu) {
            cmd = cmd.env("RUSTFLAGS", "--cfg=web_sys_unstable_apis");
        }
        cmd.run().expect("Error building example");

        cmd!(
            sh,
            "wasm-bindgen --out-dir examples/wasm/target --out-name wasm_example --target web target/wasm32-unknown-unknown/release/examples/{example}.wasm"
        )
        .run()
        .expect("Error creating wasm binding");

        if cli.optimize_size {
            cmd!(sh, "wasm-opt -Oz --output examples/wasm/target/wasm_example_bg.wasm.optimized examples/wasm/target/wasm_example_bg.wasm")
                .run().expect("Failed to optimize for size. Do you have wasm-opt correctly set up?");
        }

        if cli.test {
            let _dir = sh.push_dir(".github/start-wasm-example");
            let mut browsers = cli.browsers.clone();
            if !browsers.is_empty() {
                browsers.insert(0, "--project".to_string());
            }
            cmd!(sh, "npx playwright test --headed {browsers...}")
                .env("SCREENSHOT_PREFIX", format!("screenshot-{example}"))
                .run()
                .expect("Error running playwright test");
        }
    }
}
