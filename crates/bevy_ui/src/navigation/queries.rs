#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}
/// Camera modifiers for movement cycling.
///
/// This is only used by the cycling routine to find [`Focusable`]s at the
/// opposite side of the screen. It's expected to contain the ui camera
/// projection screen boundaries and position. See implementation of
/// [`systems::update_boundaries`](crate::systems::update_boundaries) to
/// see how to implement it yourself.
///
/// # Note
///
/// This is a [resource](Res). It is optional and will log a warning if
/// a cycling request is made and it does not exist.
#[derive(Debug)]
pub struct ScreenBoundaries {
    pub position: Vec2,
    pub screen_edge: Rect,
    pub scale: f32,
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
    boundaries: Option<Res<'w, ScreenBoundaries>>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
}
impl<'w, 's> MoveParam for UiProjectionQuery<'w, 's> {
    fn resolve_2d<'a>(
        &self,
        focused: Entity,
        direction: events::Direction,
        cycles: bool,
        siblings: &'a [Entity],
    ) -> Option<&'a Entity> {
        use events::Direction::*;

        let pos_of = |entity: Entity| {
            self.transforms
                .get(entity)
                .expect("Focusable entities must have a GlobalTransform component")
                .translation()
                .xy()
        };
        let focused_pos = pos_of(focused);
        let closest = siblings.iter().filter(|sibling| {
            direction.is_in(focused_pos, pos_of(**sibling)) && **sibling != focused
        });
        let closest = max_by_in_iter(closest, |s| -focused_pos.distance_squared(pos_of(**s)));
        match (closest, self.boundaries.as_ref()) {
            (None, None) if cycles => {
                warn!(
                    "Tried to move in {direction:?} from Focusable {focused:?} while no other \
                 Focusables were there. There were no `Res<ScreenBoundaries>`, so we couldn't \
                 compute the screen edges for cycling. Make sure you either add the \
                 bevy_ui_navigation::systems::update_boundaries system to your app or implement \
                 your own routine to manage a `Res<ScreenBoundaries>`."
                );
                None
            }
            (None, Some(boundaries)) if cycles => {
                let (x, y) = (boundaries.position.x, boundaries.position.y);
                let edge = boundaries.screen_edge;
                let scale = boundaries.scale;
                let focused_pos = match direction {
                    // NOTE: up/down axises are inverted in bevy
                    North => Vec2::new(focused_pos.x, y - scale * edge.min.y),
                    South => Vec2::new(focused_pos.x, y + scale * edge.max.y),
                    East => Vec2::new(x - edge.min.x * scale, focused_pos.y),
                    West => Vec2::new(x + edge.max.x * scale, focused_pos.y),
                };
                max_by_in_iter(siblings.iter(), |s| {
                    -focused_pos.distance_squared(pos_of(**s))
                })
            }
            (anyelse, _) => anyelse,
        }
    }
}

pub type NavigationPlugin<'w, 's> = GenericNavigationPlugin<UiProjectionQuery<'w, 's>>;

/// The navigation plugin and the default input scheme.
///
/// Add it to your app with `.add_plugins(DefaultNavigationPlugins)`.
///
/// This provides default implementations for input handling, if you want
/// your own custom input handling, you should use [`NavigationPlugin`] and
/// provide your own input handling systems.
pub struct DefaultNavigationPlugins;
impl PluginGroup for DefaultNavigationPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(GenericNavigationPlugin::<UiProjectionQuery>::new());
        group.add(DefaultNavigationSystems);
    }
}
