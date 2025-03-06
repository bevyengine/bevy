use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs::read_to_string,
    ops::Deref,
    path::Path,
    process::Command,
    sync::LazyLock,
};

use dom_query::Document;
use walkdir::WalkDir;

fn main() {
    // Generate HTML as normal
    assert!(Command::new("rustdoc")
        .args(std::env::args().skip(1))
        .status()
        .unwrap()
        .success());

    // Find package name
    let package = std::env::args()
        .skip_while(|arg| *arg != "--crate-name")
        .nth(1)
        .expect("No crate name passed")
        .clone();

    // Find output directory
    let target_dir = std::env::args()
        .skip_while(|arg| (*arg != "-o") & (*arg != "--output"))
        .nth(1)
        .unwrap()
        .clone();
    let package_dir = Path::new(&target_dir).join(package);

    // Extra style sheet is shared between files.
    // It's not shared between docs.rs packages though, so don't put it in static.files
    std::fs::write(package_dir.join("bevy_style.css"), STYLE.as_bytes()).unwrap();

    // Post-process HTML to apply our modifications
    for entry in WalkDir::new(package_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension() == Some(OsStr::new("html")) {
            let mut doc = Document::from(&*read_to_string(path).unwrap());
            add_trait_tags(&mut doc);
            add_required_components(&mut doc);
            let style_url = "../".repeat(entry.depth() - 1) + "bevy_style.css";
            doc.select("head")
                .append_html(format!("<link rel=\"stylesheet\" href=\"{style_url}\">"));
            std::fs::write(path, doc.html().as_bytes()).unwrap();
        }
    }
}

/// Adds tags indicating which core Bevy traits it implements to a type's page.
// We only use the HTML and not rustdoc's JSON output because
// we would need to be able to match Rust items to HTML files.
// There is an index from HTML to source file we could use, but
// it's in JS instead of in an easily parsable format.
// Scanning the HTML is also bit simpler as we don't need to e.g.
// manually resolver blob imports.
fn add_trait_tags(doc: &mut Document) {
    let traits = implemented_bevy_traits(doc);

    // Tags sit below headline
    let heading = doc.select("h1").first();
    heading.append_html("<div class=\"bevy-tag-container\"/>");
    let container = heading.select(".bevy-tag-container");

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
}

/// Adds information about a component's required components
fn add_required_components(doc: &mut Document) {
    // Insert as first section after main doc comment
    let next_heading = doc.select("#main-content > h2:first-of-type");

    for elem in doc.select(":has(> #required-components) > *").iter() {
        next_heading.before_html(elem.html());
    }

    // The original section nested in the trait implementation uses a smaller heading, so adjust
    doc.select("#required-components").first().rename("h2");
}

/// If this is the documentation page of a single type,
/// returns which of the relevant traits it implements,
/// alongside a (relative) url to the trait, if available.
fn implemented_bevy_traits(doc: &Document) -> HashMap<String, Option<String>> {
    // Scanning the table of contents is easiest, but we need to find the link
    // elsewhere as the ToC just points at the impls section.
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
";
