use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::{Vec2, Vec3Swizzles};
use bevy_transform::prelude::GlobalTransform;
use bevy_ui_navigation::{events::Direction, prelude::MenuNavigationStrategy};
use bevy_utils::FloatOrd;

fn is_in(direction: Direction, reference: Vec2, other: Vec2) -> bool {
    let coord = other - reference;
    use Direction::*;
    match direction {
        South => coord.y < coord.x && coord.y < -coord.x,
        North => coord.y > coord.x && coord.y > -coord.x,
        East => coord.y < coord.x && coord.y > -coord.x,
        West => coord.y > coord.x && coord.y < -coord.x,
    }
}

/// System parameter for the default cursor navigation system.
///
/// It uses the bevy [`GlobalTransform`] to compute relative positions
/// and change focus to the correct entity.
#[derive(SystemParam)]
pub struct BevyUiNavigationStrategy<'w, 's> {
    transforms: Query<'w, 's, &'static GlobalTransform>,
}
impl<'w, 's> MenuNavigationStrategy for BevyUiNavigationStrategy<'w, 's> {
    fn resolve_2d<'a>(
        &self,
        focused: Entity,
        direction: Direction,
        cycles: bool,
        siblings: &'a [Entity],
    ) -> Option<&'a Entity> {
        use Direction::*;

        let pos_of = |entity| {
            self.transforms
                .get(entity)
                .expect("Focusable entities must have a GlobalTransform component")
                .translation()
                .xy()
        };
        let focused_pos = pos_of(focused);
        let siblings_positions: Vec<_> = siblings
            .iter()
            .map(|entity| (entity, pos_of(*entity)))
            .collect();
        let closest = siblings_positions
            .iter()
            .filter(|(sibling, position)| {
                is_in(direction, focused_pos, *position) && **sibling != focused
            })
            .max_by_key(|(_, position)| FloatOrd(-focused_pos.distance_squared(*position)));

        match closest {
            None if cycles => {
                let opposite_lookup: fn(&Vec2) -> f32 = match direction {
                    East => |v| -v.x,
                    West => |v| v.x,
                    North => |v| -v.y,
                    South => |v| v.y,
                };
                let (_, opposite_pos) = siblings_positions
                    .iter()
                    .max_by_key(|(_, pos)| FloatOrd(opposite_lookup(pos)))?;
                let mut focused_pos = focused_pos;
                match direction {
                    North | South => focused_pos.y = opposite_pos.y,
                    East | West => focused_pos.x = opposite_pos.x,
                }
                siblings
                    .iter()
                    .max_by_key(|s| FloatOrd(-focused_pos.distance_squared(pos_of(**s))))
            }
            anyelse => anyelse.map(|(entity, _)| *entity),
        }
    }
}
