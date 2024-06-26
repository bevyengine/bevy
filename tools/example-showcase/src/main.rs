//! Tool to run all examples or generate a showcase page for the Bevy website.

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fmt::Display,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
    process::exit,
    thread,
    time::{Duration, Instant},
};

use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use pbr::ProgressBar;
use regex::Regex;
use toml_edit::DocumentMut;
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

        #[arg(long)]
        /// Running in CI (some adaptation to the code)
        in_ci: bool,

        #[arg(long)]
        /// Do not run stress test examples
        ignore_stress_tests: bool,

        #[arg(long)]
        /// Report execution details in files
        report_details: bool,

        #[arg(long)]
        /// Show the logs during execution
        show_logs: bool,

        #[arg(long)]
        /// File containing the list of examples to run, incompatible with pagination
        example_list: Option<String>,

        #[arg(long)]
        /// Only run examples that don't need extra features
        only_default_features: bool,
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
            in_ci,
            ignore_stress_tests,
            report_details,
            show_logs,
            example_list,
            only_default_features,
        } => {
            if example_list.is_some() && cli.page.is_some() {
                let mut cmd = Args::command();
                cmd.error(
                    ErrorKind::ArgumentConflict,
                    "example-list can't be used with pagination",
                )
                .exit();
            }
            let example_filter = example_list
                .as_ref()
                .map(|path| {
                    let file = fs::read_to_string(path).unwrap();
                    file.lines().map(|l| l.to_string()).collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let mut examples_to_run = parse_examples();

            let mut failed_examples = vec![];
            let mut successful_examples = vec![];
            let mut no_screenshot_examples = vec![];

            let mut extra_parameters = vec![];

            match (manual_stop, screenshot) {
                (true, true) => {
                    let mut file = File::create("example_showcase_config.ron").unwrap();
                    file.write_all(
                        b"(setup: (fixed_frame_time: Some(0.05)), events: [(100, Screenshot)])",
                    )
                    .unwrap();
                    extra_parameters.push("--features");
                    extra_parameters.push("bevy_ci_testing");
                }
                (true, false) => (),
                (false, true) => {
                    let mut file = File::create("example_showcase_config.ron").unwrap();
                    file.write_all(
                        b"(setup: (fixed_frame_time: Some(0.05)), events: [(100, Screenshot), (250, AppExit)])",
                    )
                    .unwrap();
                    extra_parameters.push("--features");
                    extra_parameters.push("bevy_ci_testing");
                }
                (false, false) => {
                    let mut file = File::create("example_showcase_config.ron").unwrap();
                    file.write_all(b"(events: [(250, AppExit)])").unwrap();
                    extra_parameters.push("--features");
                    extra_parameters.push("bevy_ci_testing");
                }
            }

            if in_ci {
                // Removing desktop mode as is slows down too much in CI
                let sh = Shell::new().unwrap();
                cmd!(
                    sh,
                    "git apply --ignore-whitespace tools/example-showcase/remove-desktop-app-mode.patch"
                )
                .run()
                .unwrap();

                // Don't use automatic position as it's "random" on Windows and breaks screenshot comparison
                // using the cursor position
                let sh = Shell::new().unwrap();
                cmd!(
                    sh,
                    "git apply --ignore-whitespace tools/example-showcase/fixed-window-position.patch"
                )
                .run()
                .unwrap();

                // Setting lights ClusterConfig to have less clusters by default
                // This is needed as the default config is too much for the CI runner
                cmd!(
                    sh,
                    "git apply --ignore-whitespace tools/example-showcase/reduce-light-cluster-config.patch"
                )
                .run()
                .unwrap();

                // Sending extra WindowResize events. They are not sent on CI with xvfb x11 server
                // This is needed for example split_screen that uses the window size to set the panels
                cmd!(
                    sh,
                    "git apply --ignore-whitespace tools/example-showcase/extra-window-resized-events.patch"
                )
                .run()
                .unwrap();

                // Don't try to get an audio output stream in CI as there isn't one
                // On macOS m1 runner in GitHub Actions, getting one timeouts after 15 minutes
                cmd!(
                    sh,
                    "git apply --ignore-whitespace tools/example-showcase/disable-audio.patch"
                )
                .run()
                .unwrap();

                // Sort the examples so that they are not run by category
                examples_to_run.sort_by_key(|example| {
                    let mut hasher = DefaultHasher::new();
                    example.hash(&mut hasher);
                    hasher.finish()
                });
            }

            let work_to_do = || {
                examples_to_run
                    .iter()
                    .filter(|example| example.category != "Stress Tests" || !ignore_stress_tests)
                    .filter(|example| {
                        example_list.is_none() || example_filter.contains(&example.technical_name)
                    })
                    .filter(|example| {
                        !only_default_features || example.required_features.is_empty()
                    })
                    .skip(cli.page.unwrap_or(0) * cli.per_page.unwrap_or(0))
                    .take(cli.per_page.unwrap_or(usize::MAX))
            };

            let mut pb = ProgressBar::new(work_to_do().count() as u64);

            let reports_path = "example-showcase-reports";
            if report_details {
                std::fs::create_dir(reports_path)
                    .expect("Failed to create example-showcase-reports directory");
            }

            for to_run in work_to_do() {
                let sh = Shell::new().unwrap();
                let example = &to_run.technical_name;
                let required_features = if to_run.required_features.is_empty() {
                    vec![]
                } else {
                    vec!["--features".to_string(), to_run.required_features.join(",")]
                };
                let local_extra_parameters = extra_parameters
                    .iter()
                    .map(|s| s.to_string())
                    .chain(required_features.iter().cloned())
                    .collect::<Vec<_>>();

                for command in &to_run.setup {
                    let exe = &command[0];
                    let args = &command[1..];
                    cmd!(sh, "{exe} {args...}").run().unwrap();
                }

                let _ = cmd!(
                    sh,
                    "cargo build --profile {profile} --example {example} {local_extra_parameters...}"
                ).run();
                let local_extra_parameters = extra_parameters
                    .iter()
                    .map(|s| s.to_string())
                    .chain(required_features.iter().cloned())
                    .collect::<Vec<_>>();
                let mut cmd = cmd!(
                    sh,
                    "cargo run --profile {profile} --example {example} {local_extra_parameters...}"
                );

                if let Some(backend) = wgpu_backend.as_ref() {
                    cmd = cmd.env("WGPU_BACKEND", backend);
                }

                if !manual_stop || screenshot {
                    cmd = cmd.env("CI_TESTING_CONFIG", "example_showcase_config.ron");
                }

                let before = Instant::now();
                if report_details || show_logs {
                    cmd = cmd.ignore_status();
                }
                let result = cmd.output();

                let duration = before.elapsed();

                if (!report_details && result.is_ok())
                    || (report_details && result.as_ref().unwrap().status.success())
                {
                    if screenshot {
                        let _ = fs::create_dir_all(Path::new("screenshots").join(&to_run.category));
                        let renamed_screenshot = fs::rename(
                            "screenshot-100.png",
                            Path::new("screenshots")
                                .join(&to_run.category)
                                .join(format!("{}.png", to_run.technical_name)),
                        );
                        if let Err(err) = renamed_screenshot {
                            println!("Failed to rename screenshot: {:?}", err);
                            no_screenshot_examples.push((to_run, duration));
                        } else {
                            successful_examples.push((to_run, duration));
                        }
                    } else {
                        successful_examples.push((to_run, duration));
                    }
                } else {
                    failed_examples.push((to_run, duration));
                }

                if report_details || show_logs {
                    let result = result.unwrap();
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    if show_logs {
                        println!("{}", stdout);
                        println!("{}", stderr);
                    }
                    if report_details {
                        let mut file =
                            File::create(format!("{reports_path}/{}.log", example)).unwrap();
                        file.write_all(b"==== stdout ====\n").unwrap();
                        file.write_all(stdout.as_bytes()).unwrap();
                        file.write_all(b"\n==== stderr ====\n").unwrap();
                        file.write_all(stderr.as_bytes()).unwrap();
                    }
                }

                thread::sleep(Duration::from_secs(1));
                pb.inc();
            }
            pb.finish_print("done");

            if report_details {
                let _ = fs::write(
                    format!("{reports_path}/successes"),
                    successful_examples
                        .iter()
                        .map(|(example, duration)| {
                            format!(
                                "{}/{} - {}",
                                example.category,
                                example.technical_name,
                                duration.as_secs_f32()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                let _ = fs::write(
                    format!("{reports_path}/failures"),
                    failed_examples
                        .iter()
                        .map(|(example, duration)| {
                            format!(
                                "{}/{} - {}",
                                example.category,
                                example.technical_name,
                                duration.as_secs_f32()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                if screenshot {
                    let _ = fs::write(
                        format!("{reports_path}/no_screenshots"),
                        no_screenshot_examples
                            .iter()
                            .map(|(example, duration)| {
                                format!(
                                    "{}/{} - {}",
                                    example.category,
                                    example.technical_name,
                                    duration.as_secs_f32()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
            }

            println!(
                "total: {} / passed: {}, failed: {}, no screenshot: {}",
                work_to_do().count(),
                successful_examples.len(),
                failed_examples.len(),
                no_screenshot_examples.len()
            );
            if failed_examples.is_empty() {
                println!("All examples passed!");
            } else {
                println!("Failed examples:");
                for (example, _) in failed_examples {
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

                // This beautifys the path
                // to make it a good looking URL
                // rather than having weird whitespace
                // and other characters that don't
                // work well in a URL path.
                let category_path = root_path.join(
                    to_show
                        .category
                        .replace(['(', ')'], "")
                        .replace(' ', "-")
                        .to_lowercase(),
                );

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
                let example_path = category_path.join(to_show.technical_name.replace('_', "-"));
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
# This creates redirection pages
# for the old URLs which used
# uppercase letters and whitespace.
aliases = [\"/examples{}/{}/{}\"]

[extra]
technical_name = \"{}\"
link = \"/examples{}/{}/{}\"
image = \"../static/screenshots/{}/{}.png\"
code_path = \"content/examples{}/{}\"
shader_code_paths = {:?}
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
                            match api {
                                WebApi::Webgpu => "-webgpu",
                                WebApi::Webgl2 => "",
                            },
                            to_show.category,
                            &to_show.technical_name.replace('_', "-"),
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
                            to_show.shader_paths,
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
                cmd!(
                    sh,
                    "git apply --ignore-whitespace tools/example-showcase/window-settings-wasm.patch"
                )
                .run()
                .unwrap();

                // setting the asset folder root to the root url of this domain
                cmd!(
                    sh,
                    "git apply --ignore-whitespace tools/example-showcase/asset-source-website.patch"
                )
                .run()
                .unwrap();
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
                let required_features = if to_build.required_features.is_empty() {
                    vec![]
                } else {
                    vec![
                        "--features".to_string(),
                        to_build.required_features.join(","),
                    ]
                };

                if optimize_size {
                    cmd!(
                        sh,
                        "cargo run -p build-wasm-example -- --api {api} {example} --optimize-size {required_features...}"
                    )
                    .run()
                    .unwrap();
                } else {
                    cmd!(
                        sh,
                        "cargo run -p build-wasm-example -- --api {api} {example} {required_features...}"
                    )
                    .run()
                    .unwrap();
                }

                let category_path = root_path.join(&to_build.category);
                let _ = fs::create_dir_all(&category_path);

                let example_path = category_path.join(to_build.technical_name.replace('_', "-"));
                let _ = fs::create_dir_all(&example_path);

                if website_hacks {
                    // set up the loader bar for asset loading
                    cmd!(sh, "sed -i.bak -e 's/getObject(arg0).fetch(/window.bevyLoadingBarFetch(/' -e 's/input = fetch(/input = window.bevyLoadingBarFetch(/' examples/wasm/target/wasm_example.js").run().unwrap();
                }

                let _ = fs::rename(
                    Path::new("examples/wasm/target/wasm_example.js"),
                    example_path.join("wasm_example.js"),
                );
                if optimize_size {
                    let _ = fs::rename(
                        Path::new("examples/wasm/target/wasm_example_bg.wasm.optimized"),
                        example_path.join("wasm_example_bg.wasm"),
                    );
                } else {
                    let _ = fs::rename(
                        Path::new("examples/wasm/target/wasm_example_bg.wasm"),
                        example_path.join("wasm_example_bg.wasm"),
                    );
                }
                pb.inc();
            }
            pb.finish_print("done");
        }
    }
}

fn parse_examples() -> Vec<Example> {
    let manifest_file = fs::read_to_string("Cargo.toml").unwrap();
    let manifest = manifest_file.parse::<DocumentMut>().unwrap();
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

            let source_code = fs::read_to_string(val["path"].as_str().unwrap()).unwrap();
            let shader_regex =
                Regex::new(r"(shaders\/\w+\.wgsl)|(shaders\/\w+\.frag)|(shaders\/\w+\.vert)")
                    .unwrap();

            // Find all instances of references to shader files, and keep them in an ordered and deduped vec.
            let mut shader_paths = vec![];
            for path in shader_regex
                .find_iter(&source_code)
                .map(|matches| matches.as_str().to_owned())
            {
                if !shader_paths.contains(&path) {
                    shader_paths.push(path);
                }
            }

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
                shader_paths,
                name: metadata["name"].as_str().unwrap().to_string(),
                description: metadata["description"].as_str().unwrap().to_string(),
                category: metadata["category"].as_str().unwrap().to_string(),
                wasm: metadata["wasm"].as_bool().unwrap(),
                required_features: val
                    .get("required-features")
                    .map(|rf| {
                        rf.as_array()
                            .unwrap()
                            .into_iter()
                            .map(|v| v.as_str().unwrap().to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
                setup: metadata
                    .get("setup")
                    .map(|setup| {
                        setup
                            .as_array()
                            .unwrap()
                            .into_iter()
                            .map(|v| {
                                v.as_array()
                                    .unwrap()
                                    .into_iter()
                                    .map(|v| v.as_str().unwrap().to_string())
                                    .collect()
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Data for this struct comes from both the entry for an example in the Cargo.toml file, and its associated metadata.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct Example {
    // From the example entry
    /// Name of the example, used to start it from the cargo CLI with `--example`
    technical_name: String,
    /// Path to the example file
    path: String,
    /// Path to the associated wgsl file if it exists
    shader_paths: Vec<String>,
    /// List of non default required features
    required_features: Vec<String>,
    // From the example metadata
    /// Pretty name, used for display
    name: String,
    /// Description of the example, for discoverability
    description: String,
    /// Pretty category name, matching the folder containing the example
    category: String,
    /// Does this example work in wasm?
    // TODO: be able to differentiate between WebGL2, WebGPU, both, or neither (for examples that could run on wasm without a renderer)
    wasm: bool,
    /// List of commands to run before the example. Can be used for example to specify data to download
    setup: Vec<Vec<String>>,
}
