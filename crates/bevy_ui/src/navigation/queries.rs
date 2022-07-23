use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_log::warn;
use bevy_math::{Vec2, Vec3Swizzles};
use bevy_transform::prelude::GlobalTransform;
use bevy_ui_navigation::{events::Direction, prelude::MenuNavigationStrategy};
use bevy_utils::FloatOrd;

use crate::CalculatedClip;

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
/// It uses the [`ScreenBoundaries`] resource to compute screen boundaries
/// and move the cursor accordingly when it reaches a screen border
/// in a cycling menu.
#[derive(SystemParam)]
pub struct UiProjectionQuery<'w, 's> {
    clips: Query<'w, 's, &'static CalculatedClip>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
}
impl<'w, 's> MenuNavigationStrategy for UiProjectionQuery<'w, 's> {
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
        let closest = siblings
            .iter()
            .filter(|sibling| {
                is_in(direction, focused_pos, pos_of(**sibling)) && **sibling != focused
            })
            .max_by_key(|s| FloatOrd(-focused_pos.distance_squared(pos_of(**s))));

        let boundaries = self.clips.get(focused).ok();
        match (closest, boundaries) {
            (None, None) if cycles => {
                warn!(
                    "Tried to move in {direction:?} from Focusable {focused:?} while no other Focusables were there."
                );
                None
            }
            (None, Some(CalculatedClip { clip })) if cycles => {
                let scale = 1.0;
                let focused_pos = match direction {
                    // NOTE: up/down axises are inverted in bevy
                    North => Vec2::new(focused_pos.x, scale * clip.min.y),
                    South => Vec2::new(focused_pos.x, scale * clip.max.y),
                    East => Vec2::new(clip.min.x * scale, focused_pos.y),
                    West => Vec2::new(clip.max.x * scale, focused_pos.y),
                };
                siblings
                    .iter()
                    .max_by_key(|s| FloatOrd(-focused_pos.distance_squared(pos_of(**s))))
            }
            (anyelse, _) => anyelse,
        }
    }
}
