//! A navigation framework for moving between focusable elements based on directional input.
//!
//! While virtual cursors are a common way to navigate UIs with a gamepad (or arrow keys!),
//! they are generally both slow and frustrating to use.
//! Instead, directional inputs should provide a direct way to snap between focusable elements.
//!
//! Like the rest of this crate, the [`InputFocus`] resource is manipulated to track
//! the current focus.
//!
//! Navigating between focusable entities (commonly UI nodes) is done by
//! passing a [`CompassOctant`] into the [`navigate`](DirectionalNavigation::navigate) method
//! from the [`DirectionalNavigation`] system parameter.
//!
//! Under the hood, the [`DirectionalNavigationMap`] stores a directed graph of focusable entities.
//! Each entity can have up to 8 neighbors, one for each [`CompassOctant`], balancing flexibility and required precision.
//! For now, this graph must be built manually, but in the future, it could be generated automatically.

use bevy_app::prelude::*;
use bevy_ecs::{
    entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    system::SystemParam,
};
use bevy_math::{CompassOctant, IVec2};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{prelude::*, Reflect};
use bevy_utils::HashMap;
use thiserror::Error;

use crate::InputFocus;

/// A plugin that sets up the directional navigation systems and resources.
#[derive(Default)]
pub struct DirectionalNavigationPlugin;

impl Plugin for DirectionalNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DirectionalNavigationMap>();

        #[cfg(feature = "bevy_reflect")]
        app.register_type::<NavNeighbors>()
            .register_type::<DirectionalNavigationMap>();
    }
}

/// The up-to-eight neighbors of a focusable entity, one for each [`CompassOctant`].
#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Default, Debug, PartialEq)
)]
pub struct NavNeighbors {
    /// The array of neighbors, one for each [`CompassOctant`].
    /// The mapping between array elements and directions is determined by [`CompassOctant::to_index`].
    ///
    /// If no neighbor exists in a given direction, the value will be [`None`].
    /// In most cases, using [`NavNeighbors::set`] and [`NavNeighbors::get`]
    /// will be more ergonomic than directly accessing this array.
    pub neighbors: [Option<Entity>; 8],
}

impl NavNeighbors {
    /// An empty set of neighbors.
    pub const EMPTY: NavNeighbors = NavNeighbors {
        neighbors: [None; 8],
    };

    /// Get the neighbor for a given [`CompassOctant`].
    pub const fn get(&self, octant: CompassOctant) -> Option<Entity> {
        self.neighbors[octant.to_index()]
    }

    /// Set the neighbor for a given [`CompassOctant`].
    pub const fn set(&mut self, octant: CompassOctant, entity: Entity) {
        self.neighbors[octant.to_index()] = Some(entity);
    }
}

/// A resource that stores the traversable graph of focusable entities.
///
/// Each entity can have up to 8 neighbors, one for each [`CompassOctant`].
///
/// To ensure that your graph is intuitive to navigate and generally works correctly, it should be:
///
/// - **Connected**: Every focusable entity should be reachable from every other focusable entity.
/// - **Symmetric**: If entity A is a neighbor of entity B, then entity B should be a neighbor of entity A, ideally in the reverse direction.
/// - **Physical**: The direction of navigation should match the layout of the entities when possible,
///   although looping around the edges of the screen is also acceptable.
/// - **Not self-connected**: An entity should not be a neighbor of itself; use [`None`] instead.
///
/// For now, this graph must be built manually, and the developer is responsible for ensuring that it meets the above criteria.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Resource, Debug, Default, PartialEq)
)]
pub struct DirectionalNavigationMap {
    /// A directed graph of focusable entities.
    ///
    /// Pass in the current focus as a key, and get back a collection of up to 8 neighbors,
    /// each keyed by a [`CompassOctant`].
    pub neighbors: EntityHashMap<NavNeighbors>,
}

impl DirectionalNavigationMap {
    /// Adds a new entity to the navigation map, overwriting any existing neighbors for that entity.
    ///
    /// Removes an entity from the navigation map, including all connections to and from it.
    ///
    /// Note that this is an O(n) operation, where n is the number of entities in the map,
    /// as we must iterate over each entity to check for connections to the removed entity.
    ///
    /// If you are removing multiple entities, consider using [`remove_multiple`](Self::remove_multiple) instead.
    pub fn remove(&mut self, entity: Entity) {
        self.neighbors.remove(&entity);

        for node in self.neighbors.values_mut() {
            for neighbor in node.neighbors.iter_mut() {
                if *neighbor == Some(entity) {
                    *neighbor = None;
                }
            }
        }
    }

    /// Removes a collection of entities from the navigation map.
    ///
    /// While this is still an O(n) operation, where n is the number of entities in the map,
    /// it is more efficient than calling [`remove`](Self::remove) multiple times,
    /// as we can check for connections to all removed entities in a single pass.
    ///
    /// An [`EntityHashSet`] must be provided as it is noticeably faster than the standard hasher or a [`Vec`].
    pub fn remove_multiple(&mut self, entities: EntityHashSet) {
        for entity in &entities {
            self.neighbors.remove(entity);
        }

        for node in self.neighbors.values_mut() {
            for neighbor in node.neighbors.iter_mut() {
                if let Some(entity) = *neighbor {
                    if entities.contains(&entity) {
                        *neighbor = None;
                    }
                }
            }
        }
    }

    /// Completely clears the navigation map, removing all entities and connections.
    pub fn clear(&mut self) {
        self.neighbors.clear();
    }

    /// Adds an edge between two entities in the navigation map.
    /// Any existing edge from A in the provided direction will be overwritten.
    ///
    /// The reverse edge will not be added, so navigation will only be possible in one direction.
    /// If you want to add a symmetrical edge, use [`add_symmetrical_edge`](Self::add_symmetrical_edge) instead.
    pub fn add_edge(&mut self, a: Entity, b: Entity, direction: CompassOctant) {
        self.neighbors
            .entry(a)
            .or_insert(NavNeighbors::EMPTY)
            .set(direction, b);
    }

    /// Adds a symmetrical edge between two entities in the navigation map.
    /// The A -> B path will use the provided direction, while B -> A will use the [`CompassOctant::opposite`] variant.
    ///
    /// Any existing connections between the two entities will be overwritten.
    pub fn add_symmetrical_edge(&mut self, a: Entity, b: Entity, direction: CompassOctant) {
        self.add_edge(a, b, direction);
        self.add_edge(b, a, direction.opposite());
    }

    /// Add symettrical edges between each consecutive pair of entities in the provided slice.
    ///
    /// Unlike [`add_looping_edges`](Self::add_looping_edges), this method does not loop back to the first entity.
    pub fn add_edges(&mut self, entities: &[Entity], direction: CompassOctant) {
        for i in 0..entities.len() - 1 {
            let a = entities[i];
            let b = entities[i + 1];
            self.add_symmetrical_edge(a, b, direction);
        }
    }

    /// Add symmetrical edges between each consecutive pair of entities in the provided slice, looping back to the first entity at the end.
    ///
    /// This is useful for creating a circular navigation path between a set of entities, such as a menu.
    pub fn add_looping_edges(&mut self, entities: &[Entity], direction: CompassOctant) {
        for i in 0..entities.len() {
            let a = entities[i];
            let b = entities[(i + 1) % entities.len()];
            self.add_symmetrical_edge(a, b, direction);
        }
    }

    /// Adds edges to a grid of entities, connecting each entity to its neighbors in each of the eight directions.
    ///
    /// The grid is provided as a hashmap of [`IVec2`] to entities,
    /// where the `x` represents columns from the top and `y` represents rows from the left.
    /// Missing entities will be ignored, so sparse grids are supported.
    /// If an element is missing, their neighbors will be connected to the nearest existing entity in the given direction.
    ///
    /// Diagonals travel in a purely 1:1 step fashion, so an entity at (0, 0) will connect to an entity at (2, 2) in the SouthEast direction,
    /// but would never see an entity at (1, 2).
    /// That said, you can add the same entity at multiple locations in the grid!
    /// This works well to model larger entities,
    /// even if the entities aren't truly grid-aligned.
    ///
    /// If `should_loop` is set to `true`, the top and bottom rows will be connected, as will the left and right columns.
    // The keys are specified as i32 for compatibility with taffy's grid system,
    // which uses u16 values. We need signed integers here, but we should be able to cast them losslessly.
    // PERF: This is a very flexible / "clever" implementation, but it won't be the fastest for simple cases.
    // If there's user demand for it, we can add a more optimized / less flexible version that requires non-sparse rectangular grids.
    pub fn add_grid(&mut self, entity_grid: HashMap<IVec2, Entity>, should_loop: bool) {
        let mut min_rows = i32::MAX;
        let mut min_columns = i32::MAX;
        let mut max_rows = i32::MIN;
        let mut max_columns = i32::MIN;

        for position in entity_grid.keys() {
            min_rows = min_rows.min(position.y);
            min_columns = min_columns.min(position.x);
            max_rows = max_rows.max(position.y);
            max_columns = max_columns.max(position.x);
        }

        for (&position, &entity) in &entity_grid {
            let neighbors = self.neighbors.entry(entity).or_insert(NavNeighbors::EMPTY);

            for direction in CompassOctant::VARIANTS {
                // Don't overwrite existing connections
                if neighbors.get(direction).is_some() {
                    continue;
                }

                let maybe_neighbor = if should_loop {
                    Self::find_nearest_grid_neighbor_in_direction::<true>(
                        &entity_grid,
                        min_rows,
                        min_columns,
                        max_rows,
                        max_columns,
                        direction,
                        position,
                    )
                } else {
                    Self::find_nearest_grid_neighbor_in_direction::<false>(
                        &entity_grid,
                        min_rows,
                        min_columns,
                        max_rows,
                        max_columns,
                        direction,
                        position,
                    )
                };

                if let Some(neighbor) = maybe_neighbor {
                    // PERF: we could also add the reverse edge here, to save work later
                    // Doing so is tricky with the borrow checker though, since we already have one mutable borrow alive
                    // We could construct the borrow at the last minute, but then we'd need to look up the `neighbors` entry every time
                    // and it's not clear that that would be faster
                    neighbors.set(direction, neighbor);
                }
            }
        }
    }

    /// A helper function for add_grid that finds the nearest neighbor in a given direction.
    ///
    /// Returns `Some(Entity)` if a neighbor is found, and `None` if no neighbor is found.
    /// If the entity encounters itself, it will return `None` to avoid self-connections.
    ///
    /// Only one loop in each direction is allowed, to avoid unintuitive connections.
    ///
    /// We're using a const generic to allow the compiler to optimize out the loop check if it's not needed.
    ///
    /// It is split out into its own method for ease of testing.
    fn find_nearest_grid_neighbor_in_direction<const SHOULD_LOOP: bool>(
        entity_grid: &HashMap<IVec2, Entity>,
        min_rows: i32,
        min_columns: i32,
        max_rows: i32,
        max_columns: i32,
        direction: CompassOctant,
        starting_location: IVec2,
    ) -> Option<Entity> {
        let direction_offset = direction.to_offset();
        let mut current_location = starting_location;

        if SHOULD_LOOP {
            let starting_entity = entity_grid.get(&starting_location)?;
            let mut looped_horizontal = false;
            let mut looped_vertical = false;

            loop {
                current_location += direction_offset;
                if current_location.x > max_columns {
                    if looped_horizontal {
                        break;
                    }
                    current_location.x = min_columns;
                    looped_horizontal = true;
                }
                if current_location.y > max_rows {
                    if looped_vertical {
                        break;
                    }
                    current_location.y = min_rows;
                    looped_vertical = true;
                }
                if current_location.x < min_columns {
                    if looped_horizontal {
                        break;
                    }
                    current_location.x = max_columns;
                    looped_horizontal = true;
                }
                if current_location.y < min_rows {
                    if looped_vertical {
                        break;
                    }
                    current_location.y = max_rows;
                    looped_vertical = true;
                }

                if let Some(entity) = entity_grid.get(&current_location) {
                    if entity != starting_entity {
                        return Some(*entity);
                    } else {
                        break;
                    }
                }
            }
        } else {
            loop {
                current_location += direction_offset;
                if current_location.x > max_columns || current_location.x < min_columns {
                    break;
                }
                if current_location.y > max_rows || current_location.y < min_rows {
                    break;
                }

                if let Some(entity) = entity_grid.get(&current_location) {
                    return Some(*entity);
                }
            }
        }

        None
    }

    /// Gets the entity in a given direction from the current focus, if any.
    pub fn get_neighbor(&self, focus: Entity, octant: CompassOctant) -> Option<Entity> {
        self.neighbors
            .get(&focus)
            .and_then(|neighbors| neighbors.get(octant))
    }

    /// Looks up the neighbors of a given entity.
    ///
    /// If the entity is not in the map, [`None`] will be returned.
    /// Note that the set of neighbors is not guaranteed to be non-empty though!
    pub fn get_neighbors(&self, entity: Entity) -> Option<&NavNeighbors> {
        self.neighbors.get(&entity)
    }
}

/// A system parameter for navigating between focusable entities in a directional way.
#[derive(SystemParam, Debug)]
pub struct DirectionalNavigation<'w> {
    /// The currently focused entity.
    pub focus: ResMut<'w, InputFocus>,
    /// The navigation map containing the connections between entities.
    pub map: Res<'w, DirectionalNavigationMap>,
}

impl DirectionalNavigation<'_> {
    /// Navigates to the neighbor in a given direction from the current focus, if any.
    ///
    /// Returns the new focus if successful.
    /// Returns an error if there is no focus set or if there is no neighbor in the requested direction.
    ///
    /// If the result was `Ok`, the [`InputFocus`] resource is updated to the new focus as part of this method call.
    pub fn navigate(
        &mut self,
        octant: CompassOctant,
    ) -> Result<Entity, DirectionalNavigationError> {
        if let Some(current_focus) = self.focus.0 {
            if let Some(new_focus) = self.map.get_neighbor(current_focus, octant) {
                self.focus.set(new_focus);
                Ok(new_focus)
            } else {
                Err(DirectionalNavigationError::NoNeighborInDirection)
            }
        } else {
            Err(DirectionalNavigationError::NoFocus)
        }
    }
}

/// An error that can occur when navigating between focusable entities using [directional navigation](crate::directional_navigation).
#[derive(Debug, PartialEq, Clone, Error)]
pub enum DirectionalNavigationError {
    /// No focusable entity is currently set.
    #[error("No focusable entity is currently set.")]
    NoFocus,
    /// No neighbor in the requested direction.
    #[error("No neighbor in the requested direction.")]
    NoNeighborInDirection,
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn setting_and_getting_nav_neighbors() {
        let mut neighbors = NavNeighbors::EMPTY;
        assert_eq!(neighbors.get(CompassOctant::SouthEast), None);

        neighbors.set(CompassOctant::SouthEast, Entity::PLACEHOLDER);

        for i in 0..8 {
            if i == CompassOctant::SouthEast.to_index() {
                assert_eq!(
                    neighbors.get(CompassOctant::SouthEast),
                    Some(Entity::PLACEHOLDER)
                );
            } else {
                assert_eq!(neighbors.get(CompassOctant::from_index(i).unwrap()), None);
            }
        }
    }

    #[test]
    fn simple_set_and_get_navmap() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_edge(a, b, CompassOctant::SouthEast);

        assert_eq!(map.get_neighbor(a, CompassOctant::SouthEast), Some(b));
        assert_eq!(
            map.get_neighbor(b, CompassOctant::SouthEast.opposite()),
            None
        );
    }

    #[test]
    fn symmetrical_edges() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_symmetrical_edge(a, b, CompassOctant::North);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), Some(b));
        assert_eq!(map.get_neighbor(b, CompassOctant::South), Some(a));
    }

    #[test]
    fn remove_nodes() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_edge(a, b, CompassOctant::North);
        map.add_edge(b, a, CompassOctant::South);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), Some(b));
        assert_eq!(map.get_neighbor(b, CompassOctant::South), Some(a));

        map.remove(b);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), None);
        assert_eq!(map.get_neighbor(b, CompassOctant::South), None);
    }

    #[test]
    fn remove_multiple_nodes() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_edge(a, b, CompassOctant::North);
        map.add_edge(b, a, CompassOctant::South);
        map.add_edge(b, c, CompassOctant::East);
        map.add_edge(c, b, CompassOctant::West);

        let mut to_remove = EntityHashSet::default();
        to_remove.insert(b);
        to_remove.insert(c);

        map.remove_multiple(to_remove);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), None);
        assert_eq!(map.get_neighbor(b, CompassOctant::South), None);
        assert_eq!(map.get_neighbor(b, CompassOctant::East), None);
        assert_eq!(map.get_neighbor(c, CompassOctant::West), None);
    }

    #[test]
    fn looping_edges() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_looping_edges(&[a, b, c], CompassOctant::East);

        assert_eq!(map.get_neighbor(a, CompassOctant::East), Some(b));
        assert_eq!(map.get_neighbor(b, CompassOctant::East), Some(c));
        assert_eq!(map.get_neighbor(c, CompassOctant::East), Some(a));

        assert_eq!(map.get_neighbor(a, CompassOctant::West), Some(c));
        assert_eq!(map.get_neighbor(b, CompassOctant::West), Some(a));
        assert_eq!(map.get_neighbor(c, CompassOctant::West), Some(b));
    }

    #[test]
    fn nav_with_system_param() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_looping_edges(&[a, b, c], CompassOctant::East);

        world.insert_resource(map);

        let mut focus = InputFocus::default();
        focus.set(a);
        world.insert_resource(focus);

        assert_eq!(world.resource::<InputFocus>().get(), Some(a));

        fn navigate_east(mut nav: DirectionalNavigation) {
            nav.navigate(CompassOctant::East).unwrap();
        }

        world.run_system_once(navigate_east).unwrap();
        assert_eq!(world.resource::<InputFocus>().get(), Some(b));

        world.run_system_once(navigate_east).unwrap();
        assert_eq!(world.resource::<InputFocus>().get(), Some(c));

        world.run_system_once(navigate_east).unwrap();
        assert_eq!(world.resource::<InputFocus>().get(), Some(a));
    }

    #[test]
    fn non_looping_grid_edges() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();
        let d = world.spawn_empty().id();

        let mut nav_map = DirectionalNavigationMap::default();
        // a b
        // c d
        let mut grid = HashMap::default();
        grid.insert(IVec2::new(0, 0), a);
        grid.insert(IVec2::new(1, 0), b);
        grid.insert(IVec2::new(0, 1), c);
        grid.insert(IVec2::new(1, 1), d);

        nav_map.add_grid(grid, false);

        // Cardinal directions
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::East), Some(b));
        assert_eq!(nav_map.get_neighbor(b, CompassOctant::West), Some(a));
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::South), Some(c));
        assert_eq!(nav_map.get_neighbor(c, CompassOctant::North), Some(a));
        assert_eq!(nav_map.get_neighbor(b, CompassOctant::South), Some(d));
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::North), Some(b));
        assert_eq!(nav_map.get_neighbor(c, CompassOctant::East), Some(d));
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::West), Some(c));

        // Diagonal directions
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::SouthEast), Some(d));
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::NorthWest), Some(a));
        assert_eq!(nav_map.get_neighbor(b, CompassOctant::SouthWest), Some(c));
        assert_eq!(nav_map.get_neighbor(c, CompassOctant::NorthEast), Some(b));

        // Out of bounds
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::North), None);
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::West), None);
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::NorthEast), None);
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::NorthWest), None);
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::SouthWest), None);
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::East), None);
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::South), None);
    }

    #[test]
    fn looping_grid_edges() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();
        let d = world.spawn_empty().id();

        let mut nav_map = DirectionalNavigationMap::default();
        // a b
        // c d
        let mut grid = HashMap::default();
        grid.insert(IVec2::new(0, 0), a);
        grid.insert(IVec2::new(1, 0), b);
        grid.insert(IVec2::new(0, 1), c);
        grid.insert(IVec2::new(1, 1), d);

        nav_map.add_grid(grid, false);

        // Cardinal directions
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::East), Some(b));
        assert_eq!(nav_map.get_neighbor(b, CompassOctant::West), Some(a));
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::South), Some(c));
        assert_eq!(nav_map.get_neighbor(c, CompassOctant::North), Some(a));
        assert_eq!(nav_map.get_neighbor(b, CompassOctant::South), Some(d));
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::North), Some(b));
        assert_eq!(nav_map.get_neighbor(c, CompassOctant::East), Some(d));
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::West), Some(c));

        // Diagonal directions
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::SouthEast), Some(d));
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::NorthWest), Some(a));
        assert_eq!(nav_map.get_neighbor(b, CompassOctant::SouthWest), Some(c));
        assert_eq!(nav_map.get_neighbor(c, CompassOctant::NorthEast), Some(b));

        // Out of bounds
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::North), Some(c));
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::West), Some(b));
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::NorthWest), Some(d));
        assert_eq!(nav_map.get_neighbor(a, CompassOctant::SouthWest), None);
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::East), Some(c));
        assert_eq!(nav_map.get_neighbor(d, CompassOctant::South), Some(b));
    }
}
