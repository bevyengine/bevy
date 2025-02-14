use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs::read_to_string,
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use nipper::Document;
use walkdir::WalkDir;

fn main() {
    // Use workspace dir
    let rustdoc_wrapper_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("Please run via cargo or set CARGO_MANIFEST_DIR"),
    );
    let working_dir = rustdoc_wrapper_dir.join("../..").canonicalize().unwrap();
    std::env::set_current_dir(working_dir).unwrap();

    // Generate HTML as normal
    assert!(Command::new("rustdoc")
        .args(std::env::args().skip(1))
        // Restore clobbered env var.
        // This is required by our derive macros.
        .env(
            "CARGO_MANIFEST_PATH",
            std::env::var("SET_CARGO_MANIFEST_PATH").unwrap()
        )
        .status()
        .unwrap()
        .success());

    // Find package name
    let package = std::env::args()
        .skip_while(|arg| *arg != "--crate-name")
        .nth(1)
        .expect("No crate name passed")
        .clone();

    // Post-process HTML to apply our modifications
    for entry in WalkDir::new(Path::new("target/doc/").join(package)) {
        let path = entry.unwrap();
        let path = path.path();
        if path.extension() == Some(OsStr::new("html")) {
            let mut doc = Document::from(&read_to_string(path).unwrap());
            post_process_type(&mut doc);
            std::fs::write(path, doc.html().as_bytes()).unwrap();
        }
    }
}

// We only use the HTML and not rustdoc's JSON output because
fn post_process_type(doc: &mut Document) {
    let traits = implemented_bevy_traits(doc);

    let mut heading = doc.select(".main-heading h1");
    heading.append_html("<div class=\"bevy-tag-container\"/>");
    let mut container = heading.select(".bevy-tag-container");

    for (mut tag, url) in traits {
        if (tag == "Component")
            & doc
                .select(".trait-impl.associatedtype .code-header")
                .iter()
                .any(|assoc| assoc.text().contains("type Mutability = Immutable"))
        {
            tag = "Immutable Component".to_owned()
        }

        container.append_html(format!(
            "<a class=\"bevy-tag {}-tag\" href=\"{}\">{tag}</a>",
            tag.to_lowercase(),
            url.unwrap_or_default()
        ));
    }

    doc.select("html").append_html(STYLE);
}

fn implemented_bevy_traits(doc: &Document) -> HashMap<String, Option<String>> {
    doc.select("#rustdoc-toc .trait-implementation a")
        .iter()
        .filter_map(|label| {
            let name = label.text();
            BEVY_TRAITS
                .contains(&*name)
                .then(|| ((*name).to_owned(), trait_url(doc, &name)))
        })
        .collect()
}

fn trait_url(doc: &Document, name: &str) -> Option<String> {
    let search = format!("trait.{name}.html");
    doc.select("a").iter().find_map(|a| {
        a.attr("href")
            .and_then(|url| url.ends_with(&search).then_some(url.deref().to_owned()))
    })
}

static BEVY_TRAITS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    [
        "Plugin",
        "PluginGroup",
        "Component",
        "Resource",
        "Asset",
        "Event",
        "ScheduleLabel",
        "SystemSet",
        "SystemParam",
        "Relationship",
        "RelationshipTarget",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
});

const STYLE: &str = "
<style>
    .bevy-tag-container {
        padding: 0.5rem 0;
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }

    .bevy-tag {
        display: flex;
        align-items: center;
        width: fit-content;
        height: 1.5rem;
        padding: 0 0.5rem;
        border-radius: 0.75rem;
        font-size: 1rem;
        font-weight: normal;
        color: white;
        background-color: var(--tag-color);
    }

    .component-tag,
    .immutable-component-tag {
        --tag-color: oklch(50% 27% 95);
    }

    .resource-tag {
        --tag-color: oklch(50% 27% 110);
    }

    .asset-tag {
        --tag-color: oklch(50% 27% 0);
    }

    .event-tag {
        --tag-color: oklch(50% 27% 310);
    }

    .plugin-tag,
    .plugingroup-tag {
        --tag-color: oklch(50% 27% 50);
    }

    .schedulelabel-tag,
    .systemset-tag {
        --tag-color: oklch(50% 27% 270);
    }

    .systemparam-tag {
        --tag-color: oklch(50% 27% 200);
    }

    .relationship-tag,
    .relationshiptarget-tag {
        --tag-color: oklch(50% 27% 150);
    }
</style>
";
