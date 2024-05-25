//! Tool used to build Bevy examples for wasm.

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

    #[arg(long)]
    /// Additional features to enable
    features: Vec<String>,
}

fn main() {
    let cli = Args::parse();

    assert!(!cli.examples.is_empty(), "must have at least one example");

    let default_features = true;
    let mut features: Vec<&str> = cli.features.iter().map(|f| f.as_str()).collect();
    if let Some(frames) = cli.frames {
        let mut file = File::create("ci_testing_config.ron").unwrap();
        file.write_fmt(format_args!("(events: [({frames}, AppExit)])"))
            .unwrap();
        features.push("bevy_ci_testing");
    }

    match cli.api {
        WebApi::Webgl2 => (),
        WebApi::Webgpu => {
            features.push("webgpu");
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
        let cmd = cmd!(
            sh,
            "cargo build {parameters...} --profile release --target wasm32-unknown-unknown --example {example}"
        );
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
