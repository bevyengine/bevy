//! Grouping and sorting entities based on their components.
// PERF: we could use the unique entity collection types and iterators,
// which might be faster by avoiding repeated duplication-checking.
// Might be worth doing in the future, depending on what consumers want to pass in.

use bevy_ecs::{
    archetype::ArchetypeId,
    component::ComponentId,
    entity::{Entity, EntityHashSet},
    hierarchy::{ChildOf, Children},
    name::Name,
    world::World,
};
use bevy_platform::collections::{HashMap, HashSet};

/// A tree-like grouping of entities based on their components.
///
/// This can be used to organize entities into categories and sub-categories,
/// or flattened into a single sorted list to facilitate inspection and debugging.
///
/// The most common approach is to group entities by:
/// 1. Their position in the parent-child hierarchy, and then
/// 2. Their archetype similarity, and then
/// 3. Their [`Name`] component (if present), and then
/// 4. Their [`Entity`] value.
///
/// This ensures that children of the same parent are grouped together,
/// and children with similar components are grouped together within that parent,
/// while presenting a fairly stable ordering.
///
/// To do this in a single step, you can call [`EntityGrouping::generate`]
/// with [`GroupingStrategy::Compound`].
///
/// Each of the individual grouping strategies can also be used separately,
/// using different [`GroupingStrategy`] values.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EntityGrouping {
    /// The entities that belong to this group,
    /// in the sorted order determined by the [`GroupingStrategy`] used to generate this grouping.
    pub entities: Vec<Entity>,
    /// Sub-groups within this group.
    ///
    /// Only strategies that produce sub-trees ([`GroupingStrategy::Hierarchy`],
    /// [`GroupingStrategy::ArchetypeSimilarity`], and [`GroupingStrategy::Compound`])
    /// will populate this field; the others are flat, with an empty `sub_groups` vector.
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
            GroupingStrategy::EntityValue => {
                let entities = sorted_alive_by(world, entities, |&entity| EntityKey::new(entity));
                EntityGrouping {
                    entities,
                    sub_groups: Vec::new(),
                }
            }
            GroupingStrategy::Alphabetical => {
                let entities =
                    sorted_alive_by(world, entities, |&entity| NameEntityKey::new(world, entity));
                EntityGrouping {
                    entities,
                    sub_groups: Vec::new(),
                }
            }
            GroupingStrategy::ArchetypeSimilarity => archetype_group(world, entities),
            GroupingStrategy::Hierarchy => hierarchy_group(world, entities),
            GroupingStrategy::Compound => compound_group(world, entities),
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

/// Deduplicates `entities`, drops any that are no longer alive, and sorts the rest by `key`.
///
/// This is used by the flat [`GroupingStrategy`]s over a simple [`slice::sort_by_cached_key`] call
/// to ensure that the "each alive entity appears exactly once" invariant is preserved
/// in the same fashion as the hierarchical strategies.
///
/// `key_fn` is a function that takes an `Entity` and returns a sortable key.
/// The returned vector is sorted in ascending order of the keys.
fn sorted_alive_by<F, K>(
    world: &World,
    entities: impl IntoIterator<Item = Entity>,
    key_fn: F,
) -> Vec<Entity>
where
    F: Fn(&Entity) -> K,
    K: Ord,
{
    let mut seen: EntityHashSet = EntityHashSet::default();
    let mut entities: Vec<Entity> = entities
        .into_iter()
        .filter(|&entity| seen.insert(entity) && world.get_entity(entity).is_ok())
        .collect();
    entities.sort_by_cached_key(key_fn);
    entities
}

/// Specifies what kind of grouping [`EntityGrouping::generate`] should make.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum GroupingStrategy {
    /// Group based on parent-child relationships.
    ///
    /// Parents will occur before their children, and children will be grouped under their parents.
    /// Entities at the same hierarchy level will use [`GroupingStrategy::Alphabetical`] ordering.
    Hierarchy,
    /// Group entities whose component sets are similar, hierarchically clustering by shared components.
    ///
    /// Individuals within the same archetype will be sorted by [`GroupingStrategy::Alphabetical`] ordering.
    ArchetypeSimilarity,
    /// Group by alphabetical order of [`Name`] component.
    ///
    /// Entities with no [`Name`] component are sorted after named entities,
    /// in [`GroupingStrategy::EntityValue`] order.
    Alphabetical,
    /// Group by the [`Entity`] value.
    ///
    /// This is done on the basis of the entity's index and generation,
    /// which is loosely correlated with the order of creation,
    /// though it is not guaranteed to be strictly chronological.
    ///
    /// Such an ordering is stable and unique among alive entities.
    EntityValue,
    /// Group using Bevy's standard opinionated combination of the above strategies.
    ///
    /// The tree structure follows the parent-child hierarchy, with each level's siblings ordered by
    /// archetype similarity, then by name, and finally by entity value, applied recursively.
    #[default]
    Compound,
}

/// Groups entities by their parent-child hierarchy.
///
/// Returns an [`EntityGrouping`] tree,
/// where each grouping contains one entity,
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
    let entities: EntityHashSet = entities.into_iter().collect();
    if entities.is_empty() {
        return EntityGrouping::new();
    }
    let mut root_entities = collect_root_entities(world, &entities);
    root_entities.sort_by_cached_key(|&entity| NameEntityKey::new(world, entity));
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
        included_children.sort_by_cached_key(|&entity| NameEntityKey::new(world, entity));
        tree.sub_groups = included_children
            .into_iter()
            .filter_map(|child| generate_grouping_tree(world, child, entities, visited))
            .collect();
    }
    Some(tree)
}

/// A stable, unique total ordering over [`Entity`] values,
/// matching its [`Display`](core::fmt::Display) format (`{index}v{generation}`):
/// first by index, then by generation, both ascending.
///
/// This is the ordering used by [`GroupingStrategy::EntityValue`] and as the final tie-break
/// inside [`NameEntityKey`]. Because no two currently-alive entities share an index, this is
/// a total order; the generation participates so reused entity slots order consistently.
///
/// Prefer this over [`Entity::to_bits`]: that method's opaque bit encoding is not monotonic
/// in the index and can therefore produce a reversed ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityKey {
    /// The entity's index, compared first.
    pub index: u32,
    /// The entity's generation, compared second, breaking ties between reused indices.
    pub generation: u32,
}

impl EntityKey {
    /// Generate an [`EntityKey`] from an [`Entity`].
    pub fn new(entity: Entity) -> Self {
        EntityKey {
            index: entity.index_u32(),
            generation: entity.generation().to_bits(),
        }
    }
}

/// A [`Name`]-based portion of a [`NameEntityKey`]: named entities sort before unnamed ones,
/// and named entities are compared case-insensitively.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NameKey {
    /// Whether the entity has no [`Name`]. `false` (named) sorts before `true` (unnamed).
    pub is_unnamed: bool,
    /// The lowercased [`Name`], or an empty string when the entity is unnamed.
    // PERF: we could probably make this faster by avoiding allocations here,
    // using something like `atomicow`.
    pub name: String,
}

impl NameKey {
    /// Generate a [`NameKey`] from an [`Entity`].
    pub fn new(world: &World, entity: Entity) -> Self {
        match world.get::<Name>(entity) {
            Some(name) => NameKey {
                is_unnamed: false,
                name: name.as_str().to_lowercase(),
            },
            None => NameKey {
                is_unnamed: true,
                name: String::new(),
            },
        }
    }
}

/// A human-friendly ordering over an [`Entity`]: primarily by [`Name`], then by entity value.
///
/// Named entities sort before unnamed ones, and named entities are compared case-insensitively;
/// ties fall through to [`EntityKey`].
///
/// This is the ordering used by [`GroupingStrategy::Alphabetical`], and it underpins the sibling
/// ordering of both [`GroupingStrategy::Hierarchy`] and [`GroupingStrategy::Compound`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NameEntityKey {
    /// The [`Name`]-based component.
    pub name: NameKey,
    /// The entity-value component (index, then generation).
    pub entity_value: EntityKey,
}

impl NameEntityKey {
    /// Generate a [`NameEntityKey`] from an [`Entity`].
    pub fn new(world: &World, entity: Entity) -> Self {
        NameEntityKey {
            name: NameKey::new(world, entity),
            entity_value: EntityKey::new(entity),
        }
    }
}

/// Groups entities by hierarchy, then archetype similarity, recursively.
///
/// The hierarchy is the primary grouping: parents occur before their children,
/// and each entity's sub-tree stays contiguous.
/// Within each set of siblings (the roots, or the children of the same parent),
/// entities are clustered by archetype similarity.
/// Within an archetype, entities are ordered by [`Name`], then [`Entity`] value.
///
/// As with [`hierarchy_group`], cycles or malformed hierarchies are guarded against;
/// entities involved in cycles may be omitted if no acyclic root exists.
fn compound_group(world: &World, entities: impl IntoIterator<Item = Entity>) -> EntityGrouping {
    let entities: EntityHashSet = entities.into_iter().collect();
    if entities.is_empty() {
        return EntityGrouping::new();
    }
    let root_entities = collect_root_entities(world, &entities);
    let mut visited: EntityHashSet = EntityHashSet::default();
    EntityGrouping {
        entities: Vec::new(),
        sub_groups: order_siblings(world, &root_entities, &entities, &mut visited),
    }
}

/// Returns the given siblings' compound sub-trees as a flat list, ordered by archetype similarity.
///
/// The siblings stay as direct children of their shared parent — the tree structure is purely
/// a hierarchy decision. Archetype similarity only determines the order they appear in, via the
/// leaves of the archetype-cluster tree.
fn order_siblings(
    world: &World,
    siblings: &[Entity],
    entities: &EntityHashSet,
    visited: &mut EntityHashSet,
) -> Vec<EntityGrouping> {
    if siblings.is_empty() {
        return Vec::new();
    }
    let clustered = archetype_group(world, siblings.iter().copied());
    expand_clusters(world, clustered, entities, visited)
}

/// Flattens an archetype-cluster tree into the ordered sibling sub-trees.
///
/// Cluster nodes (with empty `entities`) are traversed, while archetype leaves (with non-empty
/// `entities`) are expanded into each sibling's compound sub-tree. The cluster nesting itself is
/// discarded: only the resulting leaf order is kept, so the tree stays purely hierarchical.
fn expand_clusters(
    world: &World,
    group: EntityGrouping,
    entities: &EntityHashSet,
    visited: &mut EntityHashSet,
) -> Vec<EntityGrouping> {
    if group.entities.is_empty() {
        group
            .sub_groups
            .into_iter()
            .flat_map(|sub_group| expand_clusters(world, sub_group, entities, visited))
            .collect()
    } else {
        group
            .entities
            .into_iter()
            .filter_map(|entity| build_compound_subtree(world, entity, entities, visited))
            .collect()
    }
}

/// Builds the compound sub-tree for a single entity.
///
/// Its children are ordered by hierarchy, then archetype similarity, recursively,
/// with sibling archetypes ordered by [`Name`], then [`Entity`] value.
fn build_compound_subtree(
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
    let children: Vec<Entity> = world
        .get::<Children>(entity)
        .into_iter()
        .flat_map(|children| children.iter().copied())
        .filter(|child| entities.contains(child))
        .collect();
    Some(EntityGrouping {
        entities: vec![entity],
        sub_groups: order_siblings(world, &children, entities, visited),
    })
}

/// Sorts the entities of a single archetype by their [`NameEntityKey`]:
/// first by name (alphabetically, case-insensitive), then by [`EntityKey`].
///
/// This is the standard tie-breaker sort for this module;
/// see [`NameEntityKey`] for details and [`sorted_alive_by`] for deduplication and alive filtering
/// that should be used when sorts are applied in the context of a flat entity grouping strategy.
fn sort_entities(world: &World, entities: &mut [Entity]) {
    entities.sort_by_cached_key(|&entity| NameEntityKey::new(world, entity));
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
        sort_entities(world, &mut entities);
        grouping.entities = entities;
        return grouping;
    }
    let components_by_archetype = get_components_by_archetype(world, &entities_by_archetype);
    cluster_archetypes(world, entities_by_archetype, components_by_archetype)
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

/// Associates each archetype with the set of components it contains.
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
    world: &World,
    entities_by_archetype: HashMap<ArchetypeId, Vec<Entity>>,
    components_by_archetype: HashMap<ArchetypeId, HashSet<ComponentId>>,
) -> EntityGrouping {
    let mut clusters = seed_clusters(world, entities_by_archetype, components_by_archetype);
    while clusters.len() > 1 {
        clustering_pass(&mut clusters);
    }
    clusters.pop().expect("`clusters.len() == 1`").group
}

/// Creates one [`Cluster`] per archetype.
fn seed_clusters(
    world: &World,
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
        sort_entities(world, &mut entities);
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
    use bevy_ecs::entity::{EntityGeneration, EntityIndex};
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
    fn archetype_single_archetype_sorts_by_name() {
        let mut world = World::new();
        let first = world.spawn(Name::new("Zeta")).id();
        let second = world.spawn(Name::new("Alpha")).id();

        let grouping = archetype_group(&world, vec![second, first]);
        let expected_grouping = EntityGrouping {
            entities: vec![second, first],
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
    fn alphabetical_sorts_by_name_then_entity_value() {
        let mut world = World::new();
        let zeta = world.spawn(Name::new("Zeta")).id();
        let alpha = world.spawn(Name::new("alpha")).id();
        let alpha_dup = world.spawn(Name::new("alpha")).id();
        let unnamed = world.spawn_empty().id();

        let grouping = EntityGrouping::generate(
            &world,
            vec![unnamed, alpha_dup, zeta, alpha],
            GroupingStrategy::Alphabetical,
        );
        assert_eq!(grouping.entities, vec![alpha, alpha_dup, zeta, unnamed]);
        assert!(grouping.sub_groups.is_empty());
    }

    #[test]
    fn entity_value_sorts_by_index() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let third = world.spawn_empty().id();

        let grouping = EntityGrouping::generate(
            &world,
            vec![third, first, second],
            GroupingStrategy::EntityValue,
        );
        assert_eq!(grouping.entities, vec![first, second, third]);
        assert!(grouping.sub_groups.is_empty());
    }

    #[test]
    fn flat_strategies_dedup_and_drop_despawned() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let dead = {
            let entity = world.spawn_empty().id();
            world.despawn(entity);
            entity
        };

        // EntityValue: deduplicates and drops the despawned entity.
        let grouping = EntityGrouping::generate(
            &world,
            vec![second, first, first, dead],
            GroupingStrategy::EntityValue,
        );
        assert_eq!(grouping.entities, vec![first, second]);

        // Alphabetical: same contract.
        let grouping = EntityGrouping::generate(
            &world,
            vec![second, first, first, dead],
            GroupingStrategy::Alphabetical,
        );
        assert_eq!(grouping.entities, vec![first, second]);
    }

    #[test]
    fn entity_key_orders_by_index_then_generation() {
        // Same-index entities can't be alive simultaneously, so a real group only ever shows
        // one generation per index. Construct entities directly (no world) to deterministically
        // prove both fields participate.

        // Index is the primary key: an entity with a lower index sorts first,
        // even when its generation is higher.
        let low_index_high_gen = Entity::from_index_and_generation(
            EntityIndex::from_raw_u32(1).unwrap(),
            EntityGeneration::from_bits(5),
        );
        let high_index_low_gen = Entity::from_index_and_generation(
            EntityIndex::from_raw_u32(10).unwrap(),
            EntityGeneration::FIRST,
        );
        assert!(EntityKey::new(low_index_high_gen) < EntityKey::new(high_index_low_gen));

        // Within the same index, generation ascends.
        let same_index_low_gen = Entity::from_index_and_generation(
            EntityIndex::from_raw_u32(4).unwrap(),
            EntityGeneration::FIRST,
        );
        let same_index_high_gen = Entity::from_index_and_generation(
            EntityIndex::from_raw_u32(4).unwrap(),
            EntityGeneration::from_bits(3),
        );
        assert!(EntityKey::new(same_index_low_gen) < EntityKey::new(same_index_high_gen));
    }

    #[test]
    fn compound_groups_by_hierarchy_then_similarity() {
        let mut world = World::new();
        let parent = world.spawn(CompA).id();
        let child_zulu = world
            .spawn((Name::new("Zulu"), CompA))
            .set_parent_in_place(parent)
            .id();
        let child_alpha = world
            .spawn((Name::new("Alpha"), CompA))
            .set_parent_in_place(parent)
            .id();
        let child_bravo = world
            .spawn((Name::new("Bravo"), CompA, CompB))
            .set_parent_in_place(parent)
            .id();
        let root = world.spawn(CompA).id();

        let grouping = EntityGrouping::generate(
            &world,
            vec![child_bravo, root, child_alpha, parent, child_zulu],
            GroupingStrategy::Compound,
        );
        let flat = grouping.flatten();

        // Every entity appears exactly once.
        let mut sorted = flat.clone();
        sorted.sort_by_key(|entity| entity.index());
        let mut expected = vec![parent, root, child_alpha, child_bravo, child_zulu];
        expected.sort_by_key(|entity| entity.index());
        assert_eq!(sorted, expected);

        // Hierarchy is preserved: parents come before their children,
        // so every child follows its parent.
        let parent_pos = flat.iter().position(|&entity| entity == parent).unwrap();
        for child in [child_alpha, child_bravo, child_zulu] {
            let pos = flat.iter().position(|&entity| entity == child).unwrap();
            assert!(pos > parent_pos, "child {child:?} should follow parent");
        }

        // Entities sharing an archetype are adjacent and ordered by name.
        let pos_alpha = flat
            .iter()
            .position(|&entity| entity == child_alpha)
            .unwrap();
        let pos_zulu = flat
            .iter()
            .position(|&entity| entity == child_zulu)
            .unwrap();
        assert_eq!(pos_alpha.abs_diff(pos_zulu), 1);
        assert!(pos_alpha < pos_zulu);
    }

    #[test]
    fn compound_tree_shape_is_pure_hierarchy() {
        let mut world = World::new();
        let parent = world.spawn(CompA).id();
        let child_b = world.spawn((CompA, CompB)).set_parent_in_place(parent).id();
        let child_a = world.spawn(CompA).set_parent_in_place(parent).id();
        let root = world.spawn(CompA).id();

        let grouping = EntityGrouping::generate(
            &world,
            vec![parent, child_a, child_b, root],
            GroupingStrategy::Compound,
        );

        // The tree must be purely hierarchical: only the top-level grouping may have empty
        // `entities`; every sub-group holds exactly one entity. Archetype similarity must order
        // siblings but must not add any cluster-shell nesting.
        fn assert_pure_hierarchy(group: &EntityGrouping, is_root: bool) {
            if is_root {
                assert!(group.entities.is_empty());
            } else {
                assert_eq!(
                    group.entities.len(),
                    1,
                    "expected a single-entity hierarchy node"
                );
            }
            for sub_group in &group.sub_groups {
                assert_pure_hierarchy(sub_group, false);
            }
        }
        assert_pure_hierarchy(&grouping, true);
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
