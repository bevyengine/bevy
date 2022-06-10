use std::{fs::File, io::Write};

use clap::Parser;
use xshell::{cmd, Shell};

#[derive(Parser, Debug)]
struct Args {
    /// Examples to build
    examples: Vec<String>,

    #[clap(short, long)]
    /// Run tests
    test: bool,

    #[clap(short, long)]
    /// Run on the given browsers. By default, chromium, firefox, webkit
    browsers: Vec<String>,

    #[clap(short, long)]
    /// Stop after this number of frames
    frames: Option<usize>,
}

fn main() {
    let cli = Args::parse();
    eprintln!("{:?}", cli);

    assert!(!cli.examples.is_empty(), "must have at least one example");

    let mut bevy_ci_testing = vec![];
    if let Some(frames) = cli.frames {
        let mut file = File::create("ci_testing_config.ron").unwrap();
        file.write_fmt(format_args!("(exit_after: Some({}))", frames))
            .unwrap();
        bevy_ci_testing = vec!["--features", "bevy_ci_testing"];
    }

    for example in cli.examples {
        let sh = Shell::new().unwrap();
        let bevy_ci_testing = bevy_ci_testing.clone();
        cmd!(
            sh,
            "cargo build {bevy_ci_testing...} --release --target wasm32-unknown-unknown --example {example}"
        )
        .run()
        .expect("Error building example");
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
