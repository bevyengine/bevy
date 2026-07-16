//! Grouping and sorting entities based on their components.
// PERF: we could use the unique entity collection types and iterators,
// saving some duplicate checking.
// Might be worth doing in the future, depending on what consumers want to pass in.

use bevy_ecs::{
    archetype::ArchetypeId,
    component::ComponentId,
    entity::{Entity, EntityHashSet, EntityIndex},
    hierarchy::{ChildOf, Children},
    name::Name,
    world::World,
};
use bevy_platform::collections::{HashMap, HashSet};

/// A hierarchical grouping of entities based on their components.
///
/// This can be used to organize entities into categories and sub-categories,
/// or flattened into a single sorted list to facilitate inspection and debugging.
///
/// As discussed in [`EntityGrouping::generate`], this grouping is based on the components
/// that entities share in common.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EntityGrouping {
    /// The entities that belong to this group.
    pub entities: Vec<Entity>,
    /// Sub-groups within this group.
    pub sub_groups: Vec<EntityGrouping>,
}

impl EntityGrouping {
    /// Creates a new, empty `EntityGrouping`.
    pub const fn new() -> Self {
        Self {
            entities: Vec::new(),
            sub_groups: Vec::new(),
        }
    }

    /// Generates an [`EntityGrouping`] based on the components of the provided entities.
    pub fn generate(
        world: &World,
        entities: impl IntoIterator<Item = Entity>,
        strategy: GroupingStrategy,
    ) -> Self {
        match strategy {
            GroupingStrategy::Hierarchy => hierarchy_group(world, entities),
            GroupingStrategy::ArchetypeSimilarity => archetype_group(world, entities),
        }
    }

    /// Flattens the grouping into a single list of entities.
    ///
    /// This flattened list will represent one possible "good" ordering of the entities,
    /// where entities in the same group are kept together, and sub-groups are expanded in order.
    pub fn flatten(&self) -> Vec<Entity> {
        let mut all_entities = self.entities.clone();
        for sub_group in &self.sub_groups {
            all_entities.extend(sub_group.flatten());
        }
        all_entities
    }
}

/// Specifies what kind of grouping [`EntityGrouping::generate`] should make.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GroupingStrategy {
    /// Group based on parent-child relationships.
    #[default]
    Hierarchy,
    /// Group based how archetypes differ on which components represent them.
    ArchetypeSimilarity,
}

/// Groups entities by their parent-child hierarchy.
///
/// Returns an [`EntityGrouping`] tree,
/// Where each grouping contains one entity,
/// and the [`Children`] are stored as [`sub_groups`],
/// one for each child.
/// Children that are not included in the provided `entities` are not added.
///
/// The only exception is the root entity grouping,
/// where [`entities`] is empty,
/// and each element of [`sub_groups`] represents the root entities,
/// (entities either with no [`ChildOf`] component,
/// or whose parent isn't among the provided `entities`).
///
/// Cycles or malformed hierarchies are guarded against;
/// entities involved in cycles may be omitted if no acyclic root exists.
///
/// [`entities`]: EntityGrouping::entities
/// [`sub_groups`]: EntityGrouping::sub_groups
fn hierarchy_group(world: &World, entities: impl IntoIterator<Item = Entity>) -> EntityGrouping {
    // `HashSet` for deduplication.
    let entities: EntityHashSet = entities.into_iter().collect();
    if entities.is_empty() {
        return EntityGrouping::new();
    }
    let mut root_entities = collect_root_entities(world, &entities);
    sort_entities(world, &mut root_entities);
    let sub_groups = generate_forest(world, &entities, &root_entities);

    EntityGrouping {
        entities: Vec::new(),
        sub_groups,
    }
}

/// Returns a collection of entities
/// that either have no [`ChildOf`] component,
/// or whose [`parent`] isn't in `entities`.
///
/// [`parent`]: ChildOf::parent
fn collect_root_entities(world: &World, entities: &EntityHashSet) -> Vec<Entity> {
    entities
        .iter()
        .copied()
        .filter(|&entity| world.get_entity(entity).is_ok())
        .filter(|&entity| {
            let has_parent_in_set = world
                .get::<ChildOf>(entity)
                .is_some_and(|child_of| entities.contains(&child_of.parent()));
            !has_parent_in_set
        })
        .collect()
}

/// Generates a forest of entities,
/// where each tree is a root entity with its descendants.
fn generate_forest(
    world: &World,
    entities: &EntityHashSet,
    root_entities: &[Entity],
) -> Vec<EntityGrouping> {
    let mut visited: EntityHashSet = EntityHashSet::default();
    root_entities
        .iter()
        .filter_map(|root| generate_grouping_tree(world, *root, entities, &mut visited))
        .collect()
}

/// Returns an entity tree as an [`EntityGrouping`].
///
/// The grouping's [`entities`] only contains the provided `entity`,
/// and its [`Children`] are stored as [`sub_groups`],
/// one for each child.
///
/// Descendants of `entity` that are not in `entities` are not included.
///
/// [`entities`]: EntityGrouping::entities
/// [`sub_groups`]: EntityGrouping::sub_groups
fn generate_grouping_tree(
    world: &World,
    entity: Entity,
    entities: &EntityHashSet,
    visited: &mut EntityHashSet,
) -> Option<EntityGrouping> {
    if world.get_entity(entity).is_err() {
        return None;
    }
    if !visited.insert(entity) {
        return None;
    }
    let mut tree = EntityGrouping {
        entities: vec![entity],
        sub_groups: Vec::new(),
    };

    if let Some(children) = world.get::<Children>(entity) {
        let mut included_children: Vec<Entity> = children
            .iter()
            .filter(|child| entities.contains(*child))
            .copied()
            .collect();
        sort_entities(world, &mut included_children);
        tree.sub_groups = included_children
            .into_iter()
            .filter_map(|child| generate_grouping_tree(world, child, entities, visited))
            .collect();
    }
    Some(tree)
}

/// Sorts entities using [`sorting_key`].
fn sort_entities(world: &World, entities: &mut [Entity]) {
    entities.sort_by_cached_key(|&entity| sorting_key(world, entity));
}

/// Generates a sorting key for entities.
///
/// Three criteria are used in order,
/// with the next one being used if the comparison before has equal result:
///
/// 1. Entities with [`Name`] component are ordered before unnamed entities.
/// 2. Named entities are ordered alphabetically (case-insensitive).
/// 3. If [`Name`]s are equal, entities are ordered by [`Entity::index`].
fn sorting_key(world: &World, entity: Entity) -> (bool, String, EntityIndex) {
    match world.get::<Name>(entity) {
        Some(name) => (false, name.as_str().to_lowercase(), entity.index()),
        None => (true, String::new(), entity.index()),
    }
}

fn archetype_group(world: &World, entities: impl IntoIterator<Item = Entity>) -> EntityGrouping {
    let entities_by_archetype = get_entities_by_archetype(world, entities);
    if entities_by_archetype.is_empty() {
        return EntityGrouping::new();
    }
    if entities_by_archetype.len() == 1 {
        let mut grouping = EntityGrouping::new();
        let (_, mut entities) = entities_by_archetype
            .into_iter()
            .next()
            .expect("`entities_by_archetype.len() == 1`");
        entities.sort_by_key(|e| e.index());
        grouping.entities = entities;
        return grouping;
    }
    let components_by_archetype = get_components_by_archetype(world, &entities_by_archetype);
    cluster_archetypes(entities_by_archetype, components_by_archetype)
}

/// Associates archetypes to the entities belonging to them.
fn get_entities_by_archetype(
    world: &World,
    entities: impl IntoIterator<Item = Entity>,
) -> HashMap<ArchetypeId, Vec<Entity>> {
    let mut entities_by_archetype: HashMap<ArchetypeId, Vec<Entity>> = HashMap::default();
    let mut seen: EntityHashSet = EntityHashSet::default();
    for entity in entities {
        if !seen.insert(entity) {
            continue;
        }
        let Ok(entity_ref) = world.get_entity(entity) else {
            continue;
        };
        let archetype_id = entity_ref.archetype().id();
        entities_by_archetype
            .entry(archetype_id)
            .or_default()
            .push(entity);
    }
    entities_by_archetype
}

/// Associates components to the entities belonging to them.
fn get_components_by_archetype(
    world: &World,
    entities_by_archetype: &HashMap<ArchetypeId, Vec<Entity>>,
) -> HashMap<ArchetypeId, HashSet<ComponentId>> {
    let archetypes = world.archetypes();
    let mut archetype_ids: Vec<ArchetypeId> = entities_by_archetype.keys().cloned().collect();
    archetype_ids.sort_by_key(|archetype_id| archetype_id.index());
    let mut components_by_archetype = HashMap::default();
    for archetype_id in &archetype_ids {
        let component_set: HashSet<ComponentId> = archetypes
            .get(*archetype_id)
            .map_or_else(HashSet::default, |archetype| {
                archetype.components().iter().copied().collect()
            });
        components_by_archetype.insert(*archetype_id, component_set);
    }
    components_by_archetype
}

/// An intermediate object that helps agglomerative clustering.
#[derive(Clone)]
struct Cluster {
    /// Intersection of all archetypes inside. Functions as cache.
    signature: HashSet<ComponentId>,
    /// A transient grouping sub-tree.
    group: EntityGrouping,
}

/// Holds values for cluster distance evaluation and merging.
#[derive(Clone, Copy)]
struct ClusterPairMetadata {
    /// The `Cluster` with the lower vector index.
    low: usize,
    /// The `Cluster` with the higher vector index.
    high: usize,
    /// The distance between the two `Cluster`s.
    distance: f32,
}

/// Generates an [`EntityGrouping`] via agglomerative clustering.
fn cluster_archetypes(
    entities_by_archetype: HashMap<ArchetypeId, Vec<Entity>>,
    components_by_archetype: HashMap<ArchetypeId, HashSet<ComponentId>>,
) -> EntityGrouping {
    let mut clusters = seed_clusters(entities_by_archetype, components_by_archetype);
    while clusters.len() > 1 {
        clustering_pass(&mut clusters);
    }
    clusters.pop().expect("`clusters.len() == 1`").group
}

/// Creates one [`Cluster`] per archetype.
fn seed_clusters(
    mut entities_by_archetype: HashMap<ArchetypeId, Vec<Entity>>,
    components_by_archetype: HashMap<ArchetypeId, HashSet<ComponentId>>,
) -> Vec<Cluster> {
    let mut archetype_ids: Vec<ArchetypeId> = components_by_archetype.keys().cloned().collect();
    archetype_ids.sort_by_key(|archetype_id| archetype_id.index());
    let mut clusters: Vec<Cluster> = Vec::with_capacity(archetype_ids.len());
    for archetype_id in &archetype_ids {
        let mut entities = entities_by_archetype
            .remove(archetype_id)
            .unwrap_or_default();
        entities.sort_by_key(|entity| entity.index());
        let signature = components_by_archetype
            .get(archetype_id)
            .cloned()
            .unwrap_or_default();
        clusters.push(Cluster {
            signature,
            group: EntityGrouping {
                entities,
                sub_groups: Vec::new(),
            },
        });
    }
    clusters
}

/// Finds and merges the pair of [`Cluster`]s with the highest similarity.
fn clustering_pass(clusters: &mut Vec<Cluster>) {
    let nearest_pair = find_closest_pair(clusters);
    merge_clusters(clusters, nearest_pair);
}

/// Finds the closest pair among the given `clusters`.
fn find_closest_pair(clusters: &[Cluster]) -> ClusterPairMetadata {
    const EPSILON: f32 = 1e-5;
    let mut nearest_pair = ClusterPairMetadata {
        low: 0,
        high: 1,
        distance: f32::INFINITY,
    };
    for i in 0..clusters.len() {
        for j in (i + 1)..clusters.len() {
            let candidate_pair = ClusterPairMetadata {
                low: i,
                high: j,
                distance: jaccard_distance(&clusters[i].signature, &clusters[j].signature),
            };
            if candidate_pair.distance < nearest_pair.distance
                || ((candidate_pair.distance - nearest_pair.distance).abs() < EPSILON
                    && tie_break(candidate_pair, nearest_pair))
            {
                nearest_pair = candidate_pair;
            }
        }
    }
    nearest_pair
}

/// Merges the given `pair` among `clusters`.
fn merge_clusters(clusters: &mut Vec<Cluster>, pair: ClusterPairMetadata) {
    let right_cluster = clusters.remove(pair.high);
    let left_cluster = clusters.remove(pair.low);
    let parent_signature = left_cluster
        .signature
        .intersection(&right_cluster.signature)
        .copied()
        .collect();
    let parent_group = EntityGrouping {
        entities: Vec::new(),
        sub_groups: vec![left_cluster.group, right_cluster.group],
    };
    clusters.push(Cluster {
        signature: parent_signature,
        group: parent_group,
    });
}

/// Computes the normalized distance between two sets.
///
/// The returned value is between `0.0` and `1.0`,
/// where identical sets yield `0.0`
/// and disjoint sets yield `1.0`.
fn jaccard_distance(a: &HashSet<ComponentId>, b: &HashSet<ComponentId>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection_size = a.intersection(b).count() as f32;
    if intersection_size == 0.0 {
        return 1.0;
    }
    let union_size = (a.len() + b.len()) as f32 - intersection_size;
    1.0 - (intersection_size / union_size.max(1.0))
}

/// Determines a preference when two clusters have equal distance.
fn tie_break(pair_a: ClusterPairMetadata, pair_b: ClusterPairMetadata) -> bool {
    let key = (pair_a.low.min(pair_a.high), pair_a.low.max(pair_a.high));
    let nearest_key = (pair_b.low.min(pair_b.high), pair_b.low.max(pair_b.high));
    key < nearest_key
}

#[cfg(test)]
mod tests {
    use bevy_ecs::component::Component;
    use bevy_transform::commands::BuildChildrenTransformExt;

    use super::*;

    #[test]
    fn hierarchy_preservation() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().set_parent_in_place(a).id();
        let c = world.spawn_empty().set_parent_in_place(a).id();
        let d = world.spawn_empty().id();
        let e = world.spawn_empty().set_parent_in_place(d).id();
        let f = world.spawn_empty().set_parent_in_place(e).id();
        let g = world.spawn_empty().id();

        let grouping = hierarchy_group(&world, vec![a, b, c, d, e, f, g]);
        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![
                EntityGrouping {
                    entities: vec![a],
                    sub_groups: vec![
                        EntityGrouping {
                            entities: vec![b],
                            sub_groups: Vec::new(),
                        },
                        EntityGrouping {
                            entities: vec![c],
                            sub_groups: Vec::new(),
                        },
                    ],
                },
                EntityGrouping {
                    entities: vec![d],
                    sub_groups: vec![EntityGrouping {
                        entities: vec![e],
                        sub_groups: vec![EntityGrouping {
                            entities: vec![f],
                            sub_groups: Vec::new(),
                        }],
                    }],
                },
                EntityGrouping {
                    entities: vec![g],
                    sub_groups: Vec::new(),
                },
            ],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn hierarchy_named_vs_unnamed_sorting() {
        let mut world = World::new();
        // Root entities
        let unnamed = world.spawn_empty().id();
        let parent = world.spawn_empty().id();
        let beta = world.spawn(Name::new("Beta")).id();
        let alpha = world.spawn(Name::new("alpha")).id();
        // Children under `parent`
        let child_unnamed = world.spawn_empty().set_parent_in_place(parent).id();
        let child_named = world
            .spawn(Name::new("Child"))
            .set_parent_in_place(parent)
            .id();

        let grouping = hierarchy_group(
            &world,
            vec![unnamed, parent, beta, alpha, child_unnamed, child_named],
        );
        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![
                EntityGrouping {
                    entities: vec![alpha],
                    sub_groups: Vec::new(),
                },
                EntityGrouping {
                    entities: vec![beta],
                    sub_groups: Vec::new(),
                },
                EntityGrouping {
                    entities: vec![unnamed],
                    sub_groups: Vec::new(),
                },
                EntityGrouping {
                    entities: vec![parent],
                    sub_groups: vec![
                        EntityGrouping {
                            entities: vec![child_named],
                            sub_groups: Vec::new(),
                        },
                        EntityGrouping {
                            entities: vec![child_unnamed],
                            sub_groups: Vec::new(),
                        },
                    ],
                },
            ],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn hierarchy_unicode_sorting_case_insensitive() {
        let mut world = World::new();
        let lower_a = world.spawn(Name::new("a")).id();
        let upper_a_umlaut = world.spawn(Name::new("Ä")).id();
        let grouping = hierarchy_group(&world, vec![upper_a_umlaut, lower_a]);

        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![
                EntityGrouping {
                    entities: vec![lower_a],
                    sub_groups: Vec::new(),
                },
                EntityGrouping {
                    entities: vec![upper_a_umlaut],
                    sub_groups: Vec::new(),
                },
            ],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn hierarchy_sort_by_index() {
        let mut world = World::new();
        let first = world.spawn(Name::new("same")).id();
        let second = world.spawn(Name::new("same")).id();

        let grouping = hierarchy_group(&world, vec![second, first]);
        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![
                EntityGrouping {
                    entities: vec![first],
                    sub_groups: Vec::new(),
                },
                EntityGrouping {
                    entities: vec![second],
                    sub_groups: Vec::new(),
                },
            ],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn hierarchy_child_without_parent_in_set_becomes_root() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let child = world.spawn_empty().set_parent_in_place(parent).id();

        let grouping = hierarchy_group(&world, vec![child]);
        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![EntityGrouping {
                entities: vec![child],
                sub_groups: Vec::new(),
            }],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn hierarchy_skip_non_existent_entities() {
        let mut world = World::new();
        let alive = world.spawn_empty().id();
        let dead = world.spawn_empty().id();
        world.despawn(dead);

        let grouping = hierarchy_group(&world, vec![alive, dead]);
        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![EntityGrouping {
                entities: vec![alive],
                sub_groups: Vec::new(),
            }],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn hierarchy_deduplication() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let grouping = hierarchy_group(&world, vec![entity, entity, entity]);

        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![EntityGrouping {
                entities: vec![entity],
                sub_groups: Vec::new(),
            }],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[derive(Component)]
    struct CompA;

    #[derive(Component)]
    struct CompB;

    #[derive(Component)]
    struct CompC;

    #[test]
    fn archetype_empty_input_returns_empty_grouping() {
        let world = World::new();
        let grouping = archetype_group(&world, Vec::<Entity>::new());
        assert_eq!(grouping, EntityGrouping::new());
    }

    #[test]
    fn archetype_single_archetype_sorts_by_index() {
        let mut world = World::new();
        let first = world.spawn(Name::new("Zeta")).id();
        let second = world.spawn(Name::new("Alpha")).id();

        let grouping = archetype_group(&world, vec![second, first]);
        let expected_grouping = EntityGrouping {
            entities: vec![first, second],
            sub_groups: Vec::new(),
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn archetype_disjoint_archetypes_form_binary_tree() {
        let mut world = World::new();
        let a = world.spawn(CompA).id();
        let b = world.spawn(CompB).id();

        let grouping = archetype_group(&world, vec![a, b]);
        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![
                EntityGrouping {
                    entities: vec![a],
                    sub_groups: Vec::new(),
                },
                EntityGrouping {
                    entities: vec![b],
                    sub_groups: Vec::new(),
                },
            ],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn archetype_clusters_merge_by_similarity() {
        let mut world = World::new();
        let z = world.spawn(CompA).id();
        let y = world.spawn((CompA, CompB)).id();
        let x = world.spawn((CompA, CompB, CompC)).id();

        let grouping = archetype_group(&world, vec![z, y, x]);
        let expected_grouping = EntityGrouping {
            entities: Vec::new(),
            sub_groups: vec![
                EntityGrouping {
                    entities: vec![z],
                    sub_groups: Vec::new(),
                },
                EntityGrouping {
                    entities: Vec::new(),
                    sub_groups: vec![
                        EntityGrouping {
                            entities: vec![y],
                            sub_groups: Vec::new(),
                        },
                        EntityGrouping {
                            entities: vec![x],
                            sub_groups: Vec::new(),
                        },
                    ],
                },
            ],
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn archetype_skip_non_existent_entities() {
        let mut world = World::new();
        let alive = world.spawn(CompA).id();
        let dead = world.spawn(CompB).id();
        world.despawn(dead);

        let grouping = archetype_group(&world, vec![alive, dead]);
        let expected_grouping = EntityGrouping {
            entities: vec![alive],
            sub_groups: Vec::new(),
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn archetype_deduplication() {
        let mut world = World::new();
        let entity = world.spawn(CompA).id();

        let grouping = archetype_group(&world, vec![entity, entity, entity]);
        let expected_grouping = EntityGrouping {
            entities: vec![entity],
            sub_groups: Vec::new(),
        };
        assert_eq!(grouping, expected_grouping);
    }

    #[test]
    fn archetype_flatten_contains_all_entities_once() {
        let mut world = World::new();
        let z = world.spawn(CompA).id();
        let y = world.spawn((CompA, CompB)).id();
        let x = world.spawn((CompA, CompB, CompC)).id();

        let grouping = archetype_group(&world, vec![z, y, x, y]);
        let mut flattened = grouping.flatten();
        flattened.sort_by_key(|entity| entity.index());

        let mut expected = vec![x, y, z];
        expected.sort_by_key(|entity| entity.index());

        assert_eq!(flattened, expected);
    }

    #[test]
    fn jaccard_distance_identical_sets_is_zero() {
        let a: HashSet<ComponentId> = [ComponentId::new(0), ComponentId::new(1)]
            .into_iter()
            .collect();
        let b = a.clone();
        assert_eq!(jaccard_distance(&a, &b), 0.0);
    }

    #[test]
    fn jaccard_distance_disjoint_sets_is_one() {
        let a: HashSet<ComponentId> = [ComponentId::new(0)].into_iter().collect();
        let b: HashSet<ComponentId> = [ComponentId::new(1)].into_iter().collect();
        assert_eq!(jaccard_distance(&a, &b), 1.0);
    }

    #[test]
    fn jaccard_distance_both_empty_is_zero() {
        let a: HashSet<ComponentId> = HashSet::default();
        let b: HashSet<ComponentId> = HashSet::default();
        assert_eq!(jaccard_distance(&a, &b), 0.0);
    }

    #[test]
    fn jaccard_distance_partial_overlap_matches_expected_fraction() {
        let a: HashSet<ComponentId> = [
            ComponentId::new(0),
            ComponentId::new(1),
            ComponentId::new(2),
        ]
        .into_iter()
        .collect();
        let b: HashSet<ComponentId> = [
            ComponentId::new(1),
            ComponentId::new(2),
            ComponentId::new(3),
        ]
        .into_iter()
        .collect();
        assert_eq!(jaccard_distance(&a, &b), 0.5);
    }
}
