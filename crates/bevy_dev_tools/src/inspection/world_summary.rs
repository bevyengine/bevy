//! Condensed [inspection](super) results for an entire [`World`].
//!
//! See [`WorldSummary`] for the output, and [`WorldSummaryExt`] to generate.

use bevy_ecs::{archetype::ArchetypeId, component::ComponentId, system::Commands, world::World};
use bevy_log::info;
use bevy_utils::{memory_size::MemorySize, prelude::DebugName};
use core::{cmp::Reverse, fmt};

/// Settings for [`WorldSummary`].
#[derive(Clone, Debug, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct SummarySettings {
    /// Whether to use component names for formatting archetype signatures.
    pub include_component_names: bool,
    /// Whether to include archetypes with no entities.
    pub include_empty_archetypes: bool,
    /// Optional output limit for archetype listing.
    pub max_archetype_rows: Option<usize>,
}

impl Default for SummarySettings {
    fn default() -> Self {
        const DEFAULT_ARCHETYPE_ROWS: usize = 15;
        Self {
            include_component_names: true,
            include_empty_archetypes: false,
            max_archetype_rows: Some(DEFAULT_ARCHETYPE_ROWS),
        }
    }
}

/// Per-archetype data in an inspection summary.
#[derive(Clone, Debug)]
pub struct ArchetypeSummary {
    /// The id of this archetype.
    pub archetype_id: ArchetypeId,
    /// How many entities are in this archetype.
    pub entity_count: usize,
    /// What components define this archetype.
    pub component_ids: Vec<ComponentId>,
    /// The names of the components defining this archetype.
    ///
    /// Optional value determined by [`SummarySettings::include_component_names`].
    pub component_names: Option<Vec<DebugName>>,
    /// The combined size of this archetype's components, for a single entity.
    pub memory_size_per_entity: MemorySize,
}

impl ArchetypeSummary {
    /// Returns a human-readable archetype signature for display.
    pub fn signature_short(&self) -> String {
        match &self.component_names {
            Some(names) => {
                let mut components: Vec<String> = names
                    .iter()
                    .map(|name| name.shortname().to_string())
                    .collect();
                components.sort();
                let components_joined = components.join(", ");
                let archetype_id = self.archetype_id.index();
                format!("{components_joined} (#{archetype_id})")
            }
            None => format!("#{}", self.archetype_id.index()),
        }
    }
}

impl fmt::Display for ArchetypeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let signature = self.signature_short();
        let entity_count = self.entity_count;
        let memory_size_per_entity = self.memory_size_per_entity;
        writeln!(
            f,
            "{signature} ({entity_count} entities, {memory_size_per_entity} per entity)"
        )
    }
}

/// [`World`] data summary result.
#[derive(Clone, Debug)]
pub struct WorldSummary {
    /// The number of entities.
    pub total_entities: u32,
    /// The number of all archetypes, empty or not.
    pub total_archetypes: usize,
    /// The number of empty archetypes.
    pub empty_archetypes: usize,
    /// The number of [`Send`] resources.
    pub total_send_resources: usize,
    /// The number of non-[`Send`] resources.
    pub total_non_send_resources: usize,
    /// Information about archetypes.
    pub archetype_summaries: Vec<ArchetypeSummary>,
    /// Limit of displayed archetypes.
    max_archetype_rows: Option<usize>,
}

impl fmt::Display for WorldSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Summary:")?;
        let entity_count = self.total_entities;
        writeln!(f, "Entity count: {entity_count}")?;
        let archetype_count = self.total_archetypes;
        let empty_archetype_count = self.empty_archetypes;
        writeln!(
            f,
            "Archetype count: {archetype_count} ({empty_archetype_count} empty)"
        )?;
        let send_resource_count = self.total_send_resources;
        let non_send_resource_count = self.total_non_send_resources;
        let total_resource_count = send_resource_count + non_send_resource_count;
        writeln!(
            f,
            "Resource count: {total_resource_count} ({send_resource_count} `Send` + {non_send_resource_count} non-`Send`)"
        )?;
        writeln!(f, "Archetypes:")?;
        let archetype_display_limit = self.max_archetype_rows.unwrap_or(usize::MAX);
        for (i, archetype_summary) in self
            .archetype_summaries
            .iter()
            .take(archetype_display_limit)
            .enumerate()
        {
            let position = i + 1;
            write!(f, "{position}. {archetype_summary}")?;
        }
        if self.archetype_summaries.len() > archetype_display_limit {
            let remaining_archetypes = self.archetype_summaries.len() - archetype_display_limit;
            write!(f, "... and {remaining_archetypes} more archetypes.")?;
        }
        Ok(())
    }
}

/// Adds summary methods to [`World`].
pub trait WorldSummaryExt {
    /// Summarizes data about this [`World`].
    fn summarize(&self, settings: SummarySettings) -> WorldSummary;
}

impl WorldSummaryExt for World {
    fn summarize(&self, settings: SummarySettings) -> WorldSummary {
        let total_entities = self.entities().count_spawned();
        let total_send_resources = self.resource_entities().iter().count();
        let total_non_send_resources = self.storages().non_sends.len();
        let total_archetypes = self.archetypes().len();
        let mut archetype_summaries: Vec<ArchetypeSummary> = self
            .archetypes()
            .iter()
            .map(|archetype| ArchetypeSummary {
                archetype_id: archetype.id(),
                entity_count: archetype.entities().len(),
                component_ids: archetype.components().to_vec(),
                component_names: settings.include_component_names.then_some(
                    archetype
                        .components()
                        .iter()
                        .map(|component_id| {
                            self.components()
                                .get_name(*component_id)
                                .unwrap_or_else(|| {
                                    let component_index = component_id.index();
                                    DebugName::owned(format!("Component #{component_index}"))
                                })
                        })
                        .collect(),
                ),
                memory_size_per_entity: MemorySize::new(
                    archetype
                        .components()
                        .iter()
                        .map(|component_id| {
                            self.components()
                                .get_info(*component_id)
                                .map(|info| info.layout().size())
                                .unwrap_or(0)
                        })
                        .sum(),
                ),
            })
            .filter(|archetype_summary| {
                settings.include_empty_archetypes || archetype_summary.entity_count > 0
            })
            .collect();
        archetype_summaries.sort_by_key(|archetype_summary| {
            (
                Reverse(archetype_summary.entity_count),
                archetype_summary.component_ids.len(),
                archetype_summary.archetype_id.index(),
            )
        });
        let empty_archetypes = if settings.include_empty_archetypes {
            archetype_summaries
                .iter()
                .filter(|archetype_summary| archetype_summary.entity_count == 0)
                .count()
        } else {
            self.archetypes().len() - archetype_summaries.len()
        };
        WorldSummary {
            total_entities,
            total_archetypes,
            empty_archetypes,
            total_send_resources,
            total_non_send_resources,
            archetype_summaries,
            max_archetype_rows: settings.max_archetype_rows,
        }
    }
}

/// Adds summary methods for [`Commands`].
pub trait CommandsSummaryExt {
    /// Summarizes data about the [`World`].
    fn summarize(&mut self, settings: SummarySettings);
}

impl CommandsSummaryExt for Commands<'_, '_> {
    fn summarize(&mut self, settings: SummarySettings) {
        self.queue(move |world: &mut World| {
            let inspection_summary = world.summarize(settings);
            info!("{inspection_summary}");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::component::Component;

    #[derive(Component)]
    #[expect(dead_code, reason = "the field exists to give the component a size")]
    struct A(u32);

    #[derive(Component)]
    #[expect(dead_code, reason = "the field exists to give the component a size")]
    struct B(u64);

    fn short_names(archetype_summary: &ArchetypeSummary) -> Vec<String> {
        archetype_summary
            .component_names
            .as_ref()
            .expect("component names were requested")
            .iter()
            .map(|name| name.shortname().to_string())
            .collect()
    }

    fn summaries_with(summary: &WorldSummary, component: &str) -> Vec<ArchetypeSummary> {
        summary
            .archetype_summaries
            .iter()
            .filter(|archetype_summary| {
                short_names(archetype_summary)
                    .iter()
                    .any(|n| n == component)
            })
            .cloned()
            .collect()
    }

    #[test]
    fn fresh_world_has_no_user_entities_and_hides_empty_archetypes() {
        let world = World::new();
        let summary = world.summarize(SummarySettings::default());

        assert!(summaries_with(&summary, "A").is_empty());
        assert!(summary
            .archetype_summaries
            .iter()
            .all(|archetype_summary| archetype_summary.entity_count > 0));
        assert_eq!(
            summary.empty_archetypes,
            summary.total_archetypes - summary.archetype_summaries.len()
        );
        assert!(summary.empty_archetypes > 0);
    }

    #[test]
    fn archetype_summaries_are_sorted_by_entity_count() {
        let mut world = World::new();
        world.spawn(A(0));
        world.spawn(A(1));
        world.spawn((A(2), B(3)));

        let summary = world.summarize(SummarySettings::default());
        let with_a = summaries_with(&summary, "A");

        assert_eq!(with_a.len(), 2);
        assert_eq!(with_a[0].entity_count, 2);
        assert_eq!(with_a[1].entity_count, 1);
        assert!(short_names(&with_a[0]).contains(&"A".to_string()));
        assert!(!short_names(&with_a[0]).contains(&"B".to_string()));
    }

    #[test]
    fn empty_archetypes_can_be_included() {
        let mut world = World::new();
        let entity = world.spawn(A(0)).id();
        world.entity_mut(entity).insert(B(1));

        let settings = SummarySettings {
            include_empty_archetypes: true,
            ..Default::default()
        };
        let summary = world.summarize(settings);

        assert_eq!(summary.archetype_summaries.len(), summary.total_archetypes);
        let counted_empty = summary
            .archetype_summaries
            .iter()
            .filter(|archetype_summary| archetype_summary.entity_count == 0)
            .count();
        assert_eq!(summary.empty_archetypes, counted_empty);
        assert!(summary.empty_archetypes > 0);
    }

    #[test]
    fn signature_short_without_component_names() {
        let mut world = World::new();
        world.spawn(A(0));

        let settings = SummarySettings {
            include_component_names: false,
            ..Default::default()
        };
        let summary = world.summarize(settings);

        for archetype_summary in &summary.archetype_summaries {
            assert_eq!(
                archetype_summary.signature_short(),
                format!("#{}", archetype_summary.archetype_id.index())
            );
        }
    }

    #[test]
    fn memory_size_per_entity_sums_component_sizes() {
        let mut world = World::new();
        world.spawn((A(0), B(1)));

        let summary = world.summarize(SummarySettings::default());
        let with_a = summaries_with(&summary, "A");

        assert_eq!(with_a.len(), 1);
        assert_eq!(with_a[0].memory_size_per_entity, MemorySize::new(12));
    }

    #[test]
    fn display_respects_max_archetype_rows() {
        let mut world = World::new();
        world.spawn(A(0));
        world.spawn((A(1), B(2)));

        let settings = SummarySettings {
            max_archetype_rows: Some(1),
            ..Default::default()
        };
        let summary = world.summarize(settings);
        let hidden = summary.archetype_summaries.len() - 1;
        let display = summary.to_string();

        assert!(display.contains("1. "));
        assert!(!display.contains("2. "));
        assert!(display.contains(&format!("... and {hidden} more archetypes.")));
    }
}
