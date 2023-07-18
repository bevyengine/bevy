use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::exit,
    thread,
    time::{Duration, Instant},
};

use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use pbr::ProgressBar;
use toml_edit::Document;
use xshell::{cmd, Shell};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "release")]
    /// Compilation profile to use
    profile: String,

    #[command(subcommand)]
    action: Action,
    #[arg(long)]
    /// Pagination control - page number. To use with --per-page
    page: Option<usize>,

    #[arg(long)]
    /// Pagination control - number of examples per page. To use with --page
    per_page: Option<usize>,
}

#[derive(clap::Subcommand, Debug)]
enum Action {
    /// Run all the examples
    Run {
        #[arg(long)]
        /// WGPU backend to use
        wgpu_backend: Option<String>,

        #[arg(long)]
        /// Don't stop automatically
        manual_stop: bool,

        #[arg(long)]
        /// Take a screenshot
        screenshot: bool,
    },
    /// Build the markdown files for the website
    BuildWebsiteList {
        #[arg(long)]
        /// Path to the folder where the content should be created
        content_folder: String,

        #[arg(value_enum, long, default_value_t = WebApi::Webgpu)]
        /// Which API to use for rendering
        api: WebApi,
    },
    /// Build the examples in wasm
    BuildWasmExamples {
        #[arg(long)]
        /// Path to the folder where the content should be created
        content_folder: String,

        #[arg(long)]
        /// Enable hacks for Bevy website integration
        website_hacks: bool,

        #[arg(long)]
        /// Optimize the wasm file for size with wasm-opt
        optimize_size: bool,

        #[arg(value_enum, long)]
        /// Which API to use for rendering
        api: WebApi,
    },
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum WebApi {
    Webgl2,
    Webgpu,
}

impl Display for WebApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebApi::Webgl2 => write!(f, "webgl2"),
            WebApi::Webgpu => write!(f, "webgpu"),
        }
    }
}

fn main() {
    let cli = Args::parse();

    if cli.page.is_none() != cli.per_page.is_none() {
        let mut cmd = Args::command();
        cmd.error(
            ErrorKind::MissingRequiredArgument,
            "page and per-page must be used together",
        )
        .exit();
    }

    let profile = cli.profile;

    match cli.action {
        Action::Run {
            wgpu_backend,
            manual_stop,
            screenshot,
        } => {
            let examples_to_run = parse_examples();

            let mut failed_examples = vec![];

            let mut extra_parameters = vec![];

            match (manual_stop, screenshot) {
                (true, true) => {
                    let mut file = File::create("example_showcase_config.ron").unwrap();
                    file.write_all(
                        b"(exit_after: None, frame_time: Some(0.05), screenshot_frames: [100])",
                    )
                    .unwrap();
                    extra_parameters.push("--features");
                    extra_parameters.push("bevy_ci_testing");
                }
                (true, false) => (),
                (false, true) => {
                    let mut file = File::create("example_showcase_config.ron").unwrap();
                    file.write_all(
                        b"(exit_after: Some(300), frame_time: Some(0.05), screenshot_frames: [100])",
                    )
                    .unwrap();
                    extra_parameters.push("--features");
                    extra_parameters.push("bevy_ci_testing");
                }
                (false, false) => {
                    let mut file = File::create("example_showcase_config.ron").unwrap();
                    file.write_all(b"(exit_after: Some(300))").unwrap();
                    extra_parameters.push("--features");
                    extra_parameters.push("bevy_ci_testing");
                }
            }

            let work_to_do = || {
                examples_to_run
                    .iter()
                    .skip(cli.page.unwrap_or(0) * cli.per_page.unwrap_or(0))
                    .take(cli.per_page.unwrap_or(usize::MAX))
            };

            let mut pb = ProgressBar::new(work_to_do().count() as u64);

            for to_run in work_to_do() {
                let sh = Shell::new().unwrap();
                let example = &to_run.technical_name;
                let extra_parameters = extra_parameters.clone();
                let mut cmd = cmd!(
                    sh,
                    "cargo run --profile {profile} --example {example} {extra_parameters...}"
                );

                if let Some(backend) = wgpu_backend.as_ref() {
                    cmd = cmd.env("WGPU_BACKEND", backend);
                }

                if !manual_stop || screenshot {
                    cmd = cmd.env("CI_TESTING_CONFIG", "example_showcase_config.ron");
                }

                let before = Instant::now();

                if cmd.run().is_ok() {
                    if screenshot {
                        let _ = fs::create_dir_all(Path::new("screenshots").join(&to_run.category));
                        let _ = fs::rename(
                            "screenshot-100.png",
                            Path::new("screenshots")
                                .join(&to_run.category)
                                .join(format!("{}.png", to_run.technical_name)),
                        );
                    }
                } else {
                    failed_examples.push(to_run);
                }

                let duration = before.elapsed();
                println!("took {duration:?}");

                thread::sleep(Duration::from_secs(1));
                pb.inc();
            }
            pb.finish_print("done");
            if failed_examples.is_empty() {
                println!("All examples passed!");
            } else {
                println!("Failed examples:");
                for example in failed_examples {
                    println!(
                        "  {} / {} ({})",
                        example.category, example.name, example.technical_name
                    );
                }
                exit(1);
            }
        }
        Action::BuildWebsiteList {
            content_folder,
            api,
        } => {
            let examples_to_run = parse_examples();

            let root_path = Path::new(&content_folder);

            let _ = fs::create_dir_all(root_path);

            let mut index = File::create(root_path.join("_index.md")).unwrap();
            if matches!(api, WebApi::Webgpu) {
                index
                    .write_all(
                        "+++
title = \"Bevy Examples in WebGPU\"
template = \"examples-webgpu.html\"
sort_by = \"weight\"

[extra]
header_message = \"Examples (WebGPU)\"
+++"
                        .as_bytes(),
                    )
                    .unwrap();
            } else {
                index
                    .write_all(
                        "+++
title = \"Bevy Examples in WebGL2\"
template = \"examples.html\"
sort_by = \"weight\"

[extra]
header_message = \"Examples (WebGL2)\"
+++"
                        .as_bytes(),
                    )
                    .unwrap();
            }

            let mut categories = HashMap::new();
            for to_show in examples_to_run {
                if !to_show.wasm {
                    continue;
                }
                let category_path = root_path.join(&to_show.category);

                if !categories.contains_key(&to_show.category) {
                    let _ = fs::create_dir_all(&category_path);
                    let mut category_index = File::create(category_path.join("_index.md")).unwrap();
                    category_index
                        .write_all(
                            format!(
                                "+++
title = \"{}\"
sort_by = \"weight\"
weight = {}
+++",
                                to_show.category,
                                categories.len()
                            )
                            .as_bytes(),
                        )
                        .unwrap();
                    categories.insert(to_show.category.clone(), 0);
                }
                let example_path = category_path.join(&to_show.technical_name.replace('_', "-"));
                let _ = fs::create_dir_all(&example_path);

                let code_path = example_path.join(Path::new(&to_show.path).file_name().unwrap());
                let _ = fs::copy(&to_show.path, &code_path);

                let mut example_index = File::create(example_path.join("index.md")).unwrap();
                example_index
                    .write_all(
                        format!(
                            "+++
title = \"{}\"
template = \"example{}.html\"
weight = {}
description = \"{}\"

[extra]
technical_name = \"{}\"
link = \"/examples{}/{}/{}\"
image = \"../static/screenshots/{}/{}.png\"
code_path = \"content/examples{}/{}\"
github_code_path = \"{}\"
header_message = \"Examples ({})\"
+++",
                            to_show.name,
                            match api {
                                WebApi::Webgpu => "-webgpu",
                                WebApi::Webgl2 => "",
                            },
                            categories.get(&to_show.category).unwrap(),
                            to_show.description.replace('"', "'"),
                            &to_show.technical_name.replace('_', "-"),
                            match api {
                                WebApi::Webgpu => "-webgpu",
                                WebApi::Webgl2 => "",
                            },
                            &to_show.category,
                            &to_show.technical_name.replace('_', "-"),
                            &to_show.category,
                            &to_show.technical_name,
                            match api {
                                WebApi::Webgpu => "-webgpu",
                                WebApi::Webgl2 => "",
                            },
                            code_path
                                .components()
                                .skip(1)
                                .collect::<PathBuf>()
                                .display(),
                            &to_show.path,
                            match api {
                                WebApi::Webgpu => "WebGPU",
                                WebApi::Webgl2 => "WebGL2",
                            },
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            }
        }
        Action::BuildWasmExamples {
            content_folder,
            website_hacks,
            optimize_size,
            api,
        } => {
            let api = format!("{}", api);
            let examples_to_build = parse_examples();

            let root_path = Path::new(&content_folder);

            let _ = fs::create_dir_all(root_path);

            if website_hacks {
                // setting up the headers file for cloudflare for the correct Content-Type
                let mut headers = File::create(root_path.join("_headers")).unwrap();
                headers
                    .write_all(
                        "/*/wasm_example_bg.wasm
  Content-Type: application/wasm
"
                        .as_bytes(),
                    )
                    .unwrap();

                let sh = Shell::new().unwrap();

                // setting a canvas by default to help with integration
                cmd!(sh, "sed -i.bak 's/canvas: None,/canvas: Some(\"#bevy\".to_string()),/' crates/bevy_window/src/window.rs").run().unwrap();
                cmd!(sh, "sed -i.bak 's/fit_canvas_to_parent: false,/fit_canvas_to_parent: true,/' crates/bevy_window/src/window.rs").run().unwrap();

                // setting the asset folder root to the root url of this domain
                cmd!(sh, "sed -i.bak 's/asset_folder: \"assets\"/asset_folder: \"\\/assets\\/examples\\/\"/' crates/bevy_asset/src/lib.rs").run().unwrap();
            }

            let work_to_do = || {
                examples_to_build
                    .iter()
                    .filter(|to_build| to_build.wasm)
                    .skip(cli.page.unwrap_or(0) * cli.per_page.unwrap_or(0))
                    .take(cli.per_page.unwrap_or(usize::MAX))
            };

            let mut pb = ProgressBar::new(work_to_do().count() as u64);
            for to_build in work_to_do() {
                let sh = Shell::new().unwrap();
                let example = &to_build.technical_name;
                if optimize_size {
                    cmd!(
                        sh,
                        "cargo run -p build-wasm-example -- --api {api} {example} --optimize-size"
                    )
                    .run()
                    .unwrap();
                } else {
                    cmd!(
                        sh,
                        "cargo run -p build-wasm-example -- --api {api} {example}"
                    )
                    .run()
                    .unwrap();
                }

                let category_path = root_path.join(&to_build.category);
                let _ = fs::create_dir_all(&category_path);

                let example_path = category_path.join(&to_build.technical_name.replace('_', "-"));
                let _ = fs::create_dir_all(&example_path);

                if website_hacks {
                    // set up the loader bar for asset loading
                    cmd!(sh, "sed -i.bak -e 's/getObject(arg0).fetch(/window.bevyLoadingBarFetch(/' -e 's/input = fetch(/input = window.bevyLoadingBarFetch(/' examples/wasm/target/wasm_example.js").run().unwrap();
                }

                let _ = fs::rename(
                    Path::new("examples/wasm/target/wasm_example.js"),
                    &example_path.join("wasm_example.js"),
                );
                if optimize_size {
                    let _ = fs::rename(
                        Path::new("examples/wasm/target/wasm_example_bg.wasm.optimized"),
                        &example_path.join("wasm_example_bg.wasm"),
                    );
                } else {
                    let _ = fs::rename(
                        Path::new("examples/wasm/target/wasm_example_bg.wasm"),
                        &example_path.join("wasm_example_bg.wasm"),
                    );
                }
                pb.inc();
            }
            pb.finish_print("done");
        }
    }
}

fn parse_examples() -> Vec<Example> {
    let manifest_file = std::fs::read_to_string("Cargo.toml").unwrap();
    let manifest = manifest_file.parse::<Document>().unwrap();
    let metadatas = manifest
        .get("package")
        .unwrap()
        .get("metadata")
        .as_ref()
        .unwrap()["example"]
        .clone();

    manifest["example"]
        .as_array_of_tables()
        .unwrap()
        .iter()
        .flat_map(|val| {
            let technical_name = val.get("name").unwrap().as_str().unwrap().to_string();

            if metadatas
                .get(&technical_name)
                .and_then(|metadata| metadata.get("hidden"))
                .and_then(|hidden| hidden.as_bool())
                .and_then(|hidden| hidden.then_some(()))
                .is_some()
            {
                return None;
            }

            metadatas.get(&technical_name).map(|metadata| Example {
                technical_name,
                path: val["path"].as_str().unwrap().to_string(),
                name: metadata["name"].as_str().unwrap().to_string(),
                description: metadata["description"].as_str().unwrap().to_string(),
                category: metadata["category"].as_str().unwrap().to_string(),
                wasm: metadata["wasm"].as_bool().unwrap(),
            })
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Example {
    technical_name: String,
    path: String,
    name: String,
    description: String,
    category: String,
    wasm: bool,
}
