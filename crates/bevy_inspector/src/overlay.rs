//! A hidden inspector that opens over any app when a key is pressed.
//!
//! [`InspectorOverlayPlugin`] is part of `DefaultPlugins` when the `inspector` cargo feature is
//! enabled, so the app needs no code of its own. Its panels are not spawned until they are first
//! shown, and they render through the app's default UI camera on top of the app's UI, so a
//! camera with a viewport limits them to that viewport.

use bevy_app::{App, Plugin, Update};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    hierarchy::Children,
    name::Name,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_feathers::{
    controls::{FeathersPane, FeathersSplitPane, FeathersSplitPaneHandle},
    dark_theme::create_dark_theme,
    theme::UiTheme,
    FeathersPlugins,
};
use bevy_input::{keyboard::KeyCode, ButtonInput};
use bevy_log::warn;
use bevy_picking::Pickable;
use bevy_scene::prelude::*;
use bevy_ui::{
    percent, px, AlignItems, DefaultUiCamera, FlexDirection, GlobalZIndex, Node, PositionType, Val,
};
use bevy_ui_widgets::split_pane_self_update;

use crate::{
    details_panel::details_panel,
    entity_tree::{entity_tree_panel, InspectorUi},
    InspectorPlugin,
};

/// Adds the inspector to an app as an overlay, hidden until [`toggle_key`](Self::toggle_key) is
/// pressed.
///
/// While the overlay is hidden, the inspector does not walk the world.
pub struct InspectorOverlayPlugin {
    /// The key that shows and hides the overlay.
    pub toggle_key: KeyCode,
}

impl Default for InspectorOverlayPlugin {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F12,
        }
    }
}

impl Plugin for InspectorOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FeathersPlugins, InspectorPlugin))
            .insert_resource(InspectorOverlayKey(self.toggle_key))
            .add_systems(Update, toggle_inspector_overlay);
    }
}

#[derive(Resource)]
struct InspectorOverlayKey(KeyCode);

/// Marker for the root entity of the inspector overlay.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct InspectorOverlay;

fn toggle_inspector_overlay(
    key: Res<InspectorOverlayKey>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut overlays: Query<&mut Visibility, With<InspectorOverlay>>,
    mut theme: ResMut<UiTheme>,
    ui_camera: DefaultUiCamera,
    mut commands: Commands,
) {
    if !keys.is_some_and(|keys| keys.just_pressed(key.0)) {
        return;
    }
    if let Ok(mut visibility) = overlays.single_mut() {
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
        return;
    }
    if theme.0.token_assignments.is_empty() {
        theme.0 = create_dark_theme();
    }
    if ui_camera.get().is_none() {
        warn!("The inspector overlay renders through the default UI camera, but there is none.");
    }
    commands.spawn_scene(overlay_scene());
}

fn overlay_scene() -> impl Scene {
    bsn! {
        InspectorOverlay
        InspectorUi
        Name::new("Inspector")
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            padding: px(12),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
        }
        GlobalZIndex(i32::MAX)
        Pickable::IGNORE
        Children [
            @FeathersSplitPane
            Node {
                width: px(720),
                max_height: percent(100),
            }
            on(split_pane_self_update)
            Children [
                @FeathersPane { @size: 1.0, @min_size: 220.0 }
                Children [
                    @entity_tree_panel()
                    Node {
                        width: Val::Auto,
                    }
                ]
                --
                @FeathersSplitPaneHandle
                --
                @FeathersPane { @size: 2.0, @min_size: 240.0 }
                Children [
                    @details_panel()
                    Node {
                        width: Val::Auto,
                    }
                ]
            ]
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity_tree::{EntityTreeSync, InspectorRow, InspectorTreeView},
        inspector_visible,
    };
    use bevy_app::TaskPoolPlugin;
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::{
        entity::Entity, hierarchy::ChildOf, name::Name, prelude::*, system::RunSystemOnce,
    };
    use bevy_feathers::theme::UiTheme;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            bevy_time::TimePlugin,
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
            InspectorPlugin,
        ));
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app.init_resource::<UiTheme>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(InspectorOverlayKey(KeyCode::F12));
        app.add_systems(Update, toggle_inspector_overlay);
        app
    }

    fn press(app: &mut App) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::F12);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::F12);
        keys.clear();
    }

    fn overlays(app: &mut App) -> Vec<Entity> {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<InspectorOverlay>>()
            .iter(world)
            .collect()
    }

    fn row_count(app: &mut App, source: Entity) -> usize {
        let world = app.world_mut();
        world
            .query::<&InspectorRow>()
            .iter(world)
            .filter(|row| row.source == source)
            .count()
    }

    #[test]
    fn nothing_is_spawned_until_the_key_is_pressed() {
        let mut app = test_app();
        app.update();
        app.update();
        assert!(overlays(&mut app).is_empty());
        assert!(!app.world_mut().run_system_once(inspector_visible).unwrap());
        assert!(app
            .world()
            .resource::<UiTheme>()
            .0
            .token_assignments
            .is_empty());
    }

    #[test]
    fn the_key_shows_and_hides_the_overlay() {
        let mut app = test_app();
        press(&mut app);
        let overlay = overlays(&mut app)[0];
        assert!(!app
            .world()
            .resource::<UiTheme>()
            .0
            .token_assignments
            .is_empty());
        assert!(app.world_mut().run_system_once(inspector_visible).unwrap());

        press(&mut app);
        assert_eq!(overlays(&mut app), [overlay]);
        assert_eq!(
            app.world().get::<Visibility>(overlay),
            Some(&Visibility::Hidden)
        );
        assert!(!app.world_mut().run_system_once(inspector_visible).unwrap());

        press(&mut app);
        assert_eq!(overlays(&mut app), [overlay]);
        assert_eq!(
            app.world().get::<Visibility>(overlay),
            Some(&Visibility::Inherited)
        );
    }

    #[test]
    fn rows_are_current_on_the_frame_the_overlay_opens() {
        let mut app = test_app();
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        app.update();
        assert_eq!(row_count(&mut app, subject), 0);
        press(&mut app);
        assert_eq!(row_count(&mut app, subject), 1);
    }

    #[test]
    fn hidden_overlay_does_not_sync_and_catches_up_when_shown() {
        let mut app = test_app();
        press(&mut app);
        press(&mut app);
        let subject = app.world_mut().spawn(Name::new("Late")).id();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        assert_eq!(row_count(&mut app, subject), 0);
        press(&mut app);
        assert_eq!(row_count(&mut app, subject), 1);
    }

    #[test]
    fn keeps_the_apps_theme() {
        let mut app = test_app();
        let mut theme = create_dark_theme();
        theme.semantic_base.clear();
        app.insert_resource(UiTheme(theme));
        press(&mut app);
        assert!(app.world().resource::<UiTheme>().0.semantic_base.is_empty());
    }

    #[test]
    fn plugin_can_be_added_more_than_once() {
        let mut app = test_app();
        app.add_plugins(InspectorPlugin);
        app.update();
    }

    #[test]
    fn a_hidden_ancestor_hides_a_panel() {
        let mut world = World::new();
        let root = world.spawn(Visibility::Hidden).id();
        let panel = world.spawn((InspectorTreeView, ChildOf(root))).id();
        assert!(!world.run_system_once(inspector_visible).unwrap());

        world.entity_mut(root).insert(Visibility::Inherited);
        assert!(world.run_system_once(inspector_visible).unwrap());

        world.entity_mut(panel).insert(Visibility::Hidden);
        assert!(!world.run_system_once(inspector_visible).unwrap());
    }
}
