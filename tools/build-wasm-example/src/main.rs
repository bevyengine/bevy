use std::{fs::File, io::Write};

use clap::Parser;
use xshell::{cmd, Shell};

#[derive(Parser, Debug)]
struct Args {
    /// Examples to build
    #[clap(value_parser)]
    examples: Vec<String>,

    #[clap(short, long, value_parser)]
    /// Run tests
    test: bool,

    #[clap(long, value_parser)]
    /// Build for webgpu
    webgpu: bool,

    #[clap(short, long, value_parser)]
    /// Run on the given browsers. By default, chromium, firefox, webkit
    browsers: Vec<String>,

    #[clap(short, long, value_parser)]
    /// Stop after this number of frames
    frames: Option<usize>,
}

fn main() {
    let cli = Args::parse();

    assert!(!cli.examples.is_empty(), "must have at least one example");

    let mut features = vec![];
    if let Some(frames) = cli.frames {
        let mut file = File::create("ci_testing_config.ron").unwrap();
        file.write_fmt(format_args!("(exit_after: Some({}))", frames))
            .unwrap();
        features.push("bevy_ci_testing");
    }

    if cli.webgpu {
        features.push("webgpu");
        println!("/!\\ /!\\ /!\\ /!\\ /!\\");
        println!("You enabled the webgpu feature.");
        println!("For it to work, you need to manually edit the root Cargo.toml to remove the webgl feature.");
        println!("/!\\ /!\\ /!\\ /!\\ /!\\");
    }

    for example in cli.examples {
        let sh = Shell::new().unwrap();
        // let features = features.clone();
        let features_string = features.join(",");
        let features = if !features.is_empty() {
            vec!["--features", &features_string]
        } else {
            vec![]
        };
        let mut cmd = cmd!(
            sh,
            "cargo build {features...} --release --target wasm32-unknown-unknown --example {example}"
        );
        if cli.webgpu {
            cmd = cmd.env("RUSTFLAGS", "--cfg=web_sys_unstable_apis");
        }
        cmd.run().expect("Error building example");
        cmd!(
            sh,
            "wasm-bindgen --out-dir examples/wasm/target --out-name wasm_example --target web target/wasm32-unknown-unknown/release/examples/{example}.wasm"
        )
        .run()
        .expect("Error creating wasm binding");

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
