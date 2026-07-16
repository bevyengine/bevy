//! Tools to fuzzily map a component or resource name to its corresponding ID.
//!
//! This is useful for GUI and text-based inspection tools,
//! where users may want to search for components or resources by name,
//! but may not know the exact spelling or formatting of the name.
//!
//! The underlying string similarity logic relies on the [`strsim`] crate.
//! Matching uses Jaro-Winkler similarity to find the closest match,
//! and is case-insensitive and ignores leading/trailing whitespace.
//! Jaro-Winkler distance is a more suitable metric for this type of fuzzy matching
//! than Levenshtein distance, as it is more robust to transpositions and prioritizes prefix characters.

use bevy_ecs::component::ComponentId;
use bevy_ecs::world::World;
use strsim::jaro_winkler;

/// Attempts to find a [`ComponentId`] for the given fuzzy component name.
///
/// A best-effort match will be returned,
/// or `None` if no suitable match could be found.
///
/// See [`fuzzy_resource_name_to_id`] for a similar function for resources.
///
/// Only the "shortname" of the component (i.e., without module paths) is considered.
///
/// The `threshold` parameter controls the minimum similarity score required for a match to be included in the results.
/// When incrementally searching for a matching name, it may be useful to start with a lower threshold
/// and increase it as the user types more characters.
pub fn fuzzy_component_name_to_id(
    world: &World,
    fuzzy_name: &str,
    threshold: f64,
) -> Vec<(f64, ComponentId)> {
    let candidates = world.components().iter_registered().map(|info| info.id());
    fuzzy_name_to_id(world, fuzzy_name, candidates, threshold)
}

/// Attempts to find a [`ComponentId`] for the given fuzzy resource name.
///
/// A best-effort match will be returned,
/// or `None` if no suitable match could be found.
///
/// See [`fuzzy_component_name_to_id`] for a similar function for components.
///
/// Only the "shortname" of the resource (i.e., without module paths) is considered.
///
/// The `threshold` parameter controls the minimum similarity score required for a match to be included in the results.
/// When incrementally searching for a matching name, it may be useful to start with a lower threshold
/// and increase it as the user types more characters.
pub fn fuzzy_resource_name_to_id(
    world: &World,
    fuzzy_name: &str,
    threshold: f64,
) -> Vec<(f64, ComponentId)> {
    // We can restrict the candidate set to the component id values that are registered as resources,
    // allowing us to share code with the component equivalent above.
    let candidates = world.resource_entities().iter().map(|(id, _)| id);
    fuzzy_name_to_id(world, fuzzy_name, candidates, threshold)
}

/// Finds the best fuzzy match for `fuzzy_name` among the provided candidate [`ComponentId`]s.
///
/// Matching uses Jaro-Winkler similarity over each candidate's "shortname",
/// which trims module paths.
///
/// This is normalized by trimming whitespace and converting to lowercase.
/// An exact (post-normalization) match short-circuits and is always preferred.
///
/// The `threshold` parameter controls the minimum similarity score required for a match to be included in the results.
fn fuzzy_name_to_id(
    world: &World,
    fuzzy_name: &str,
    candidates: impl Iterator<Item = ComponentId>,
    threshold: f64,
) -> Vec<(f64, ComponentId)> {
    let processed_fuzzy_name = fuzzy_name.trim().to_lowercase();

    // PERF: it is almost certainly more efficient to build an accelerated structure
    // across all possible names once, rather than re-computing distances
    // whenever a user enters a new fuzzy name.
    let mut matches = Vec::with_capacity(5);
    for id in candidates {
        let Some(name) = world.components().get_name(id) else {
            continue;
        };
        let processed_name = name.shortname().to_string().trim().to_lowercase();

        if processed_fuzzy_name == processed_name {
            return vec![(1.0, id)];
        }
        let similarity = jaro_winkler(&processed_fuzzy_name, &processed_name);
        if similarity >= threshold {
            matches.push((similarity, id));
        }
    }

    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    matches
}
