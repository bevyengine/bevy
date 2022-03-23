use crate::{CalculatedClip, Node};
use bevy_core::FloatOrd;
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    reflect::ReflectComponent,
    system::{Local, Query, Res, Resource},
};
use bevy_input::{mouse::MouseButton, touch::Touches, Input};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_window::Windows;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Describes what type of input interaction has occurred for a UI node.
///
/// This is commonly queried with a `Changed<Interaction>` filter.
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect_value(Component, Serialize, Deserialize, PartialEq)]
pub enum Interaction {
    /// The node has been clicked
    Clicked,
    /// The node has been hovered over
    Hovered,
    /// Nothing has happened
    None,
}

impl Default for Interaction {
    fn default() -> Self {
        Interaction::None
    }
}

/// Describes whether the node should block interactions with lower nodes
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect_value(Component, Serialize, Deserialize, PartialEq)]
pub enum FocusPolicy {
    /// Blocks interaction
    Block,
    /// Lets interaction pass through
    Pass,
}

impl Default for FocusPolicy {
    fn default() -> Self {
        FocusPolicy::Block
    }
}

/// Contains entities whose Interaction should be set to None
#[derive(Default)]
pub struct State {
    entities_to_reset: SmallVec<[Entity; 1]>,
}

/// The system that sets Interaction for all UI elements based on the mouse cursor activity
#[allow(clippy::type_complexity)]
pub fn ui_focus_system(
    state: Local<State>,
    windows: Res<Windows>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    node_query: Query<(
        Entity,
        &Node,
        &GlobalTransform,
        Option<&mut Interaction>,
        Option<&FocusPolicy>,
        Option<&CalculatedClip>,
    )>,
) {
    focus_ui(
        state,
        windows,
        mouse_button_input,
        touches_input,
        node_query,
    )
}

#[allow(clippy::type_complexity)]
fn focus_ui<Cursor: CursorResource>(
    mut state: Local<State>,
    windows: Res<Cursor>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    mut node_query: Query<(
        Entity,
        &Node,
        &GlobalTransform,
        Option<&mut Interaction>,
        Option<&FocusPolicy>,
        Option<&CalculatedClip>,
    )>,
) {
    let cursor_position = windows.get_cursor_position();

    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(mut interaction) = node_query.get_component_mut::<Interaction>(entity) {
            *interaction = Interaction::None;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.just_released(0);
    if mouse_released {
        for (_entity, _node, _global_transform, interaction, _focus_policy, _clip) in
            node_query.iter_mut()
        {
            if let Some(mut interaction) = interaction {
                if *interaction == Interaction::Clicked {
                    *interaction = Interaction::None;
                }
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.just_released(0);

    let mut moused_over_z_sorted_nodes = node_query
        .iter_mut()
        .filter_map(
            |(entity, node, global_transform, interaction, focus_policy, clip)| {
                let position = global_transform.translation;
                let ui_position = position.truncate();
                let extents = node.size / 2.0;
                let mut min = ui_position - extents;
                let mut max = ui_position + extents;
                if let Some(clip) = clip {
                    min = Vec2::max(min, clip.clip.min);
                    max = Vec2::min(max, clip.clip.max);
                }
                // if the current cursor position is within the bounds of the node, consider it for
                // clicking
                let contains_cursor = if let Some(cursor_position) = cursor_position {
                    (min.x..max.x).contains(&cursor_position.x)
                        && (min.y..max.y).contains(&cursor_position.y)
                } else {
                    false
                };

                if contains_cursor {
                    Some((entity, focus_policy, interaction, FloatOrd(position.z)))
                } else {
                    if let Some(mut interaction) = interaction {
                        if *interaction == Interaction::Hovered
                            || (cursor_position.is_none() && *interaction != Interaction::None)
                        {
                            *interaction = Interaction::None;
                        }
                    }
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    moused_over_z_sorted_nodes.sort_by_key(|(_, _, _, z)| -*z);

    let mut moused_over_z_sorted_nodes = moused_over_z_sorted_nodes.into_iter();
    // set Clicked or Hovered on top nodes
    for (entity, focus_policy, interaction, _) in moused_over_z_sorted_nodes.by_ref() {
        if let Some(mut interaction) = interaction {
            if mouse_clicked {
                // only consider nodes with Interaction "clickable"
                if *interaction != Interaction::Clicked {
                    *interaction = Interaction::Clicked;
                    // if the mouse was simultaneously released, reset this Interaction in the next
                    // frame
                    if mouse_released {
                        state.entities_to_reset.push(entity);
                    }
                }
            } else if *interaction == Interaction::None {
                *interaction = Interaction::Hovered;
            }
        }

        match focus_policy.cloned().unwrap_or(FocusPolicy::Block) {
            FocusPolicy::Block => {
                break;
            }
            FocusPolicy::Pass => { /* allow the next node to be hovered/clicked */ }
        }
    }
    // reset lower nodes to None
    for (_entity, _focus_policy, interaction, _) in moused_over_z_sorted_nodes {
        if let Some(mut interaction) = interaction {
            if *interaction != Interaction::None {
                *interaction = Interaction::None;
            }
        }
    }
}

trait CursorResource: Resource {
    fn get_cursor_position(&self) -> Option<Vec2>;
}

impl CursorResource for Windows {
    fn get_cursor_position(&self) -> Option<Vec2> {
        self.get_primary()
            .and_then(|window| window.cursor_position())
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_ecs::event::Events;
    use bevy_ecs::prelude::ParallelSystemDescriptorCoercion;
    use bevy_ecs::query::Changed;
    use bevy_input::touch::{touch_screen_input_system, TouchInput, TouchPhase};
    use bevy_math::Vec3;

    use super::*;

    const NODE_SIZE: f32 = 5.0;

    #[test]
    fn test_sets_hovered_nodes() {
        let test_sets = vec![
            vec![(None, Interaction::None)],
            vec![(Some((0., 0.)), Interaction::None)],
            vec![(Some((10., 10.)), Interaction::Hovered)],
            vec![
                (Some((10., 10.)), Interaction::Hovered),
                (Some((0., 0.)), Interaction::None),
            ],
            vec![
                (Some((10., 10.)), Interaction::Hovered),
                (None, Interaction::None),
            ],
        ];

        for test_set in test_sets {
            test_hovered_nodes(test_set);
        }
    }

    fn test_hovered_nodes(test_set: Vec<(Option<(f32, f32)>, Interaction)>) {
        let mut app = TestApp::new();
        let entity = app.spawn_node_entity_at(10., 10.);

        for (cursor_position, expected_interaction) in test_set {
            app.set_cursor_position(cursor_position);

            app.run_step();

            let interaction = app.get_interaction(entity);
            assert_eq!(
                &expected_interaction, interaction,
                "for position {:?}",
                cursor_position,
            );
        }
    }

    #[test]
    fn test_sets_clicked_nodes() {
        let test_sets_mouse = vec![
            vec![(None, Interaction::None)],
            vec![(Some((0., 0.)), Interaction::None)],
            vec![(Some((10., 10.)), Interaction::Clicked)],
            vec![
                (Some((10., 10.)), Interaction::Clicked),
                (Some((0., 0.)), Interaction::Clicked),
            ],
            vec![
                (Some((10., 10.)), Interaction::Clicked),
                (None, Interaction::None),
            ],
        ];
        let test_sets_touch = vec![
            vec![(None, Interaction::None)],
            vec![(Some((0., 0.)), Interaction::None)],
            vec![(Some((10., 10.)), Interaction::Clicked)],
            vec![
                (Some((10., 10.)), Interaction::Clicked),
                (Some((0., 0.)), Interaction::None),
            ],
            vec![
                (Some((10., 10.)), Interaction::Clicked),
                (None, Interaction::None),
            ],
        ];

        for mouse_test_set in test_sets_mouse {
            test_clicked_nodes(mouse_test_set, false);
        }
        for touch_test_set in test_sets_touch {
            test_clicked_nodes(touch_test_set, true);
        }
    }

    fn test_clicked_nodes(test_set: Vec<(Option<(f32, f32)>, Interaction)>, touch: bool) {
        let mut app = TestApp::new();
        let entity = app.spawn_node_entity_at(10., 10.);

        for (cursor_position, expected_interaction) in test_set {
            app.set_cursor_position(cursor_position);
            if touch {
                app.set_screen_touched();
            } else {
                app.set_mouse_clicked();
            }

            app.run_step();

            let interaction = app.get_interaction(entity);
            assert_eq!(
                &expected_interaction, interaction,
                "for position {:?}",
                cursor_position,
            );
        }
    }

    #[test]
    fn test_sets_click_stacked_nodes() {
        let test_sets = vec![
            (None, Interaction::None),
            (Some(FocusPolicy::Block), Interaction::None),
            (Some(FocusPolicy::Pass), Interaction::Clicked),
        ];

        for (focus_policy, expected_interaction) in test_sets {
            test_click_stacked_nodes(focus_policy, expected_interaction);
        }
    }

    fn test_click_stacked_nodes(
        focus_policy: Option<FocusPolicy>,
        expected_interaction: Interaction,
    ) {
        let mut app = TestApp::new();
        let background_entity = app.spawn_node_entity_with_z_at(10., 10., 0., focus_policy);
        let foreground_entity = app.spawn_node_entity_with_z_at(10., 10., 5., focus_policy);

        app.set_cursor_position(Some((10., 10.)));
        app.set_mouse_clicked();

        app.run_step();

        assert_eq!(
            &Interaction::Clicked,
            app.get_interaction(foreground_entity)
        );
        assert_eq!(
            &expected_interaction,
            app.get_interaction(background_entity)
        );
    }

    #[test]
    fn hover_one_node_then_click_the_other_where_both_overlap() {
        let mut app = TestApp::new();
        let background_node_position = 8.;
        let background_entity = app.spawn_node_entity_with_z_at(
            background_node_position,
            background_node_position,
            0.,
            Some(FocusPolicy::Block),
        );
        let foreground_entity =
            app.spawn_node_entity_with_z_at(10., 10., 5., Some(FocusPolicy::Block));

        app.set_cursor_position(Some((6., 6.)));

        app.run_step();

        assert_eq!(&Interaction::None, app.get_interaction(foreground_entity));
        assert_eq!(
            &Interaction::Hovered,
            app.get_interaction(background_entity)
        );

        app.set_cursor_position(Some((background_node_position, background_node_position)));
        app.set_mouse_clicked();

        app.run_step();

        assert_eq!(
            &Interaction::Clicked,
            app.get_interaction(foreground_entity)
        );
        assert_eq!(&Interaction::None, app.get_interaction(background_entity));
    }

    #[test]
    fn click_then_move_away_and_release_mouse_button() {
        let mut app = TestApp::new();
        let entity = app.spawn_node_entity_at(10., 10.);

        app.set_cursor_position(Some((10., 10.)));
        app.set_mouse_clicked();

        app.run_step();
        assert_eq!(&Interaction::Clicked, app.get_interaction(entity));

        app.set_cursor_position(Some((0., 0.)));

        app.run_step();
        assert_eq!(&Interaction::Clicked, app.get_interaction(entity));

        app.set_mouse_released();

        app.run_step();
        assert_eq!(&Interaction::None, app.get_interaction(entity));
    }

    #[test]
    fn click_and_keep_pressed() {
        let mut app = TestApp::new();
        let entity = app.spawn_node_entity_at(10., 10.);
        app.set_cursor_position(Some((10., 10.)));
        app.set_mouse_clicked();

        app.run_step();
        assert_eq!(&Interaction::Clicked, app.get_interaction(entity));

        app.run_step();
        assert_eq!(&Interaction::Clicked, app.get_interaction(entity));
    }

    #[test]
    fn click_and_release() {
        let mut app = TestApp::new();
        let entity = app.spawn_node_entity_at(10., 10.);
        app.set_cursor_position(Some((10., 10.)));

        app.set_mouse_clicked();
        app.run_step();
        assert_eq!(&Interaction::Clicked, app.get_interaction(entity));

        app.set_mouse_released();
        app.run_step();
        assert_eq!(&Interaction::Hovered, app.get_interaction(entity));
    }

    #[test]
    fn click_and_release_in_single_frame() {
        let mut app = TestApp::new();
        let entity = app.spawn_node_entity_at(10., 10.);
        app.set_cursor_position(Some((10., 10.)));

        app.set_mouse_clicked();
        app.set_mouse_released();
        app.run_step();
        assert_eq!(&Interaction::Clicked, app.get_interaction(entity));

        app.run_step();
        assert_eq!(&Interaction::Hovered, app.get_interaction(entity));
    }

    #[test]
    fn change_detection_journey() {
        let mut app = TestApp::new();
        app.spawn_node_entity_at(10., 10.);

        app.run_step();

        app.expect_no_changed_interaction("mouse does still not touch target");
        app.run_step();

        app.set_cursor_position(Some((10., 10.)));
        app.expect_changed_interaction("mouse hovers target");
        app.run_step();

        app.expect_no_changed_interaction("mouse still hovers target");
        app.run_step();

        app.set_mouse_clicked();
        app.expect_changed_interaction("mouse clicked target");
        app.run_step();

        app.expect_no_changed_interaction("mouse button still clicked");
        app.run_step();

        app.set_cursor_position(Some((0., 0.)));
        app.expect_no_changed_interaction("mouse dragged away, but button still clicked");
        app.run_step();
    }

    struct TestApp {
        app: App,
    }

    impl TestApp {
        fn new() -> TestApp {
            let mut app = App::new();
            app.init_resource::<Input<MouseButton>>()
                .init_resource::<Touches>()
                .init_resource::<WindowsDouble>()
                .init_resource::<ChangedInteractionExpectation>()
                .add_event::<TouchInput>()
                .add_system(touch_screen_input_system.before("under_test"))
                .add_system(focus_ui::<WindowsDouble>.label("under_test"))
                .add_system(watch_changes.after("under_test"));

            TestApp { app }
        }

        fn set_cursor_position(&mut self, cursor_position: Option<(f32, f32)>) {
            let cursor_position = cursor_position.map(|(x, y)| Vec2::new(x, y));
            self.app.insert_resource(WindowsDouble { cursor_position });
        }

        fn set_screen_touched(&mut self) {
            self.app
                .world
                .get_resource_mut::<Events<TouchInput>>()
                .unwrap()
                .send(TouchInput {
                    phase: TouchPhase::Ended,
                    position: Default::default(),
                    force: None,
                    id: 0,
                })
        }

        fn set_mouse_clicked(&mut self) {
            let mut mouse_input = self
                .app
                .world
                .get_resource_mut::<Input<MouseButton>>()
                .unwrap();
            mouse_input.press(MouseButton::Left);
        }

        fn set_mouse_released(&mut self) {
            let mut mouse_input = self
                .app
                .world
                .get_resource_mut::<Input<MouseButton>>()
                .unwrap();
            mouse_input.release(MouseButton::Left);
        }

        fn spawn_node_entity_at(&mut self, x: f32, y: f32) -> Entity {
            self.app
                .world
                .spawn()
                .insert(GlobalTransform {
                    translation: Vec3::new(x, y, 0.0),
                    ..GlobalTransform::default()
                })
                .insert(Node {
                    size: Vec2::new(NODE_SIZE, NODE_SIZE),
                })
                .insert(Interaction::None)
                .id()
        }

        fn spawn_node_entity_with_z_at(
            &mut self,
            x: f32,
            y: f32,
            z: f32,
            focus_policy: Option<FocusPolicy>,
        ) -> Entity {
            let mut entity = self.app.world.spawn();
            if let Some(focus_policy) = focus_policy {
                entity.insert(focus_policy);
            }

            entity
                .insert(GlobalTransform {
                    translation: Vec3::new(x, y, z),
                    ..GlobalTransform::default()
                })
                .insert(Node {
                    size: Vec2::new(NODE_SIZE, NODE_SIZE),
                })
                .insert(Interaction::None)
                .id()
        }

        fn run_step(&mut self) {
            self.app.schedule.run_once(&mut self.app.world);

            let mut mouse_input = self
                .app
                .world
                .get_resource_mut::<Input<MouseButton>>()
                .unwrap();
            mouse_input.clear();
        }

        fn get_interaction(&self, entity: Entity) -> &Interaction {
            self.app.world.get::<Interaction>(entity).unwrap()
        }

        fn expect_changed_interaction(&mut self, message: &str) {
            let mut changed_interaction_expectation = self
                .app
                .world
                .get_resource_mut::<ChangedInteractionExpectation>()
                .unwrap();
            changed_interaction_expectation.0 = Some(true);
            changed_interaction_expectation.1 = message.to_string();
        }

        fn expect_no_changed_interaction(&mut self, message: &str) {
            let mut changed_interaction_expectation = self
                .app
                .world
                .get_resource_mut::<ChangedInteractionExpectation>()
                .unwrap();
            changed_interaction_expectation.0 = Some(false);
            changed_interaction_expectation.1 = message.to_string();
        }
    }

    #[derive(Default)]
    struct ChangedInteractionExpectation(Option<bool>, String);

    fn watch_changes(
        query: Query<Entity, Changed<Interaction>>,
        expected_changed_interaction: Res<ChangedInteractionExpectation>,
    ) {
        match expected_changed_interaction.0 {
            Some(true) => assert!(
                query.iter().count() > 0,
                "{}",
                expected_changed_interaction.1.as_str()
            ),
            Some(false) => assert_eq!(
                query.iter().count(),
                0,
                "{}",
                expected_changed_interaction.1.as_str()
            ),
            None => {}
        }
    }

    #[derive(Debug, Default)]
    struct WindowsDouble {
        cursor_position: Option<Vec2>,
    }

    impl CursorResource for WindowsDouble {
        fn get_cursor_position(&self) -> Option<Vec2> {
            self.cursor_position
        }
    }
}
