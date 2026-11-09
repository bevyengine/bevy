//! A hidden inspector that opens over any app when a key is pressed.
//!
//! [`InspectorOverlayPlugin`] is part of `DefaultPlugins` when the `inspector` cargo feature is
//! enabled, so the app needs no code of its own. Its panels are not spawned until they are first
//! shown, and they render through the app's default UI camera, so a camera with a viewport limits
//! them to that viewport. A draggable toolbar shows and hides each panel, and the panels are
//! floating dialogs, so they can also be dragged and closed.

use bevy_app::{App, Plugin, PreUpdate, Update};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    event::EntityEvent,
    hierarchy::Children,
    name::Name,
    observer::On,
    query::{Changed, With},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_feathers::{
    constants::icons,
    containers::subpane_header,
    controls::{ButtonVariant, FeathersDialogClose, FeathersToolButton},
    dark_theme::create_dark_theme,
    display::{caption, icon},
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, UiTheme},
    tokens, FeathersPlugins,
};
use bevy_input::{keyboard::KeyCode, ButtonInput, InputSystems};
use bevy_log::warn;
use bevy_picking::{cursor::EntityCursor, Pickable, PickingSystems};
use bevy_scene::prelude::*;
use bevy_ui::{
    percent, px, AlignItems, BorderRadius, DefaultUiCamera, FlexDirection, Node, PositionType,
    UiRect, Val,
};
use bevy_ui_widgets::{Activate, Dialog, DialogDragHandle, RequestClose};
use bevy_window::SystemCursorIcon;

use crate::{
    details_panel::{details_body, DetailsCollapsed},
    entity_tree::{entity_tree_body, InspectorUi},
    InspectorPlugin,
};

/// Adds the inspector to an app as an overlay, hidden until [`toggle_key`](Self::toggle_key) is
/// pressed.
///
/// The inspector only walks the world while the entity tree panel is showing.
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
            .add_systems(
                PreUpdate,
                toggle_inspector_overlay
                    .after(InputSystems)
                    .before(PickingSystems::Last),
            )
            .add_systems(Update, sync_panel_toggles);
        app.world_mut()
            .resource_mut::<DetailsCollapsed>()
            .by_default = true;
    }
}

#[derive(Resource)]
struct InspectorOverlayKey(KeyCode);

/// Marker for the root entity of the inspector overlay.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct InspectorOverlay;

/// A panel of the inspector overlay, shown and hidden independently from the toolbar.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorOverlayPanel {
    /// The entity tree.
    #[default]
    Tree,
    /// The details of the selected entity.
    Details,
}

/// A toolbar button that shows and hides a panel.
#[derive(Component, Clone, Copy, Debug, Default)]
struct PanelToggle(InspectorOverlayPanel);

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
        *visibility = flipped(*visibility);
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

fn flipped(visibility: Visibility) -> Visibility {
    match visibility {
        Visibility::Hidden => Visibility::Inherited,
        _ => Visibility::Hidden,
    }
}

fn on_panel_toggle(
    activate: On<Activate>,
    toggles: Query<&PanelToggle>,
    mut panels: Query<(&InspectorOverlayPanel, &mut Visibility)>,
) {
    let Ok(toggle) = toggles.get(activate.event_target()) else {
        return;
    };
    for (panel, mut visibility) in &mut panels {
        if *panel == toggle.0 {
            *visibility = flipped(*visibility);
        }
    }
}

fn on_panel_close(mut close: On<RequestClose>, mut visibilities: Query<&mut Visibility>) {
    close.propagate(false);
    if let Ok(mut visibility) = visibilities.get_mut(close.event_target()) {
        *visibility = Visibility::Hidden;
    }
}

fn sync_panel_toggles(
    panels: Query<(&InspectorOverlayPanel, &Visibility), Changed<Visibility>>,
    mut toggles: Query<(&PanelToggle, &mut ButtonVariant)>,
) {
    for (panel, visibility) in &panels {
        for (toggle, mut variant) in &mut toggles {
            if toggle.0 == *panel {
                variant.set_if_neq(ButtonVariant::selected(*visibility != Visibility::Hidden));
            }
        }
    }
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
        }
        Pickable::IGNORE
        Children [
            @toolbar()
            --
            @panel_window(px(12), px(220))
            InspectorOverlayPanel::Tree
            Children [
                @panel_header("Entities")
                --
                @entity_tree_body()
            ]
            --
            @panel_window(px(240), px(280))
            InspectorOverlayPanel::Details
            Children [
                @panel_header("Details")
                --
                @details_body()
            ]
        ]
    }
}

fn toolbar() -> impl Scene {
    bsn! {
        Dialog
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(6),
            padding: UiRect::all(px(3)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
        }
        ThemeBackgroundColor(tokens::DIALOG_BG)
        ThemeBorderColor(tokens::DIALOG_BORDER)
        InheritableThemeTextColor(tokens::DIALOG_TEXT)
        Children [
            @FeathersToolButton {
                @caption: bsn! { @caption("Entities") },
                @variant: {ButtonVariant::Primary},
            }
            PanelToggle(InspectorOverlayPanel::Tree)
            on(on_panel_toggle)
            --
            @FeathersToolButton {
                @caption: bsn! { @caption("Details") },
                @variant: {ButtonVariant::Primary},
            }
            PanelToggle(InspectorOverlayPanel::Details)
            on(on_panel_toggle)
            --
            DialogDragHandle
            EntityCursor::System(SystemCursorIcon::Grab)
            Node {
                align_items: AlignItems::Center,
            }
            Children [
                @icon(icons::GRIP_VERTICAL)
            ]
        ]
    }
}

fn panel_window(left: Val, width: Val) -> impl Scene {
    bsn! {
        Dialog
        Node {
            position_type: PositionType::Absolute,
            left: {left},
            top: px(52),
            width: {width},
            max_height: percent(90),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
        }
        ThemeBackgroundColor(tokens::DIALOG_BG)
        ThemeBorderColor(tokens::DIALOG_BORDER)
        InheritableThemeTextColor(tokens::DIALOG_TEXT)
        on(on_panel_close)
    }
}

fn panel_header(title: &'static str) -> impl Scene {
    bsn! {
        @subpane_header()
        DialogDragHandle
        Children [
            @caption(title)
            --
            @FeathersDialogClose
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        details_panel_visible,
        entity_tree::{EntityTreeSync, InspectorRow, InspectorTreeView},
        entity_tree_visible,
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
        app.add_systems(
            Update,
            (toggle_inspector_overlay, sync_panel_toggles).chain(),
        );
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
        assert!(!app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
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
        assert!(app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());

        press(&mut app);
        assert_eq!(overlays(&mut app), [overlay]);
        assert_eq!(
            app.world().get::<Visibility>(overlay),
            Some(&Visibility::Hidden)
        );
        assert!(!app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());

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

    fn panel(app: &mut App, kind: InspectorOverlayPanel) -> Entity {
        let world = app.world_mut();
        world
            .query::<(Entity, &InspectorOverlayPanel)>()
            .iter(world)
            .find(|(_, panel)| **panel == kind)
            .unwrap()
            .0
    }

    fn toggle_button(app: &mut App, kind: InspectorOverlayPanel) -> Entity {
        let world = app.world_mut();
        world
            .query::<(Entity, &PanelToggle)>()
            .iter(world)
            .find(|(_, toggle)| toggle.0 == kind)
            .unwrap()
            .0
    }

    fn click(app: &mut App, button: Entity) {
        app.world_mut().trigger(Activate { entity: button });
        app.update();
    }

    #[test]
    fn toolbar_toggles_each_panel_independently() {
        let mut app = test_app();
        press(&mut app);
        let tree = panel(&mut app, InspectorOverlayPanel::Tree);
        let details = panel(&mut app, InspectorOverlayPanel::Details);
        let button = toggle_button(&mut app, InspectorOverlayPanel::Tree);
        assert_eq!(
            app.world().get::<ButtonVariant>(button),
            Some(&ButtonVariant::Primary)
        );

        click(&mut app, button);
        assert_eq!(
            app.world().get::<Visibility>(tree),
            Some(&Visibility::Hidden)
        );
        assert_ne!(
            app.world().get::<Visibility>(details),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(button),
            Some(&ButtonVariant::Normal)
        );
        assert!(!app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
        assert!(app
            .world_mut()
            .run_system_once(details_panel_visible)
            .unwrap());

        click(&mut app, button);
        assert_ne!(
            app.world().get::<Visibility>(tree),
            Some(&Visibility::Hidden)
        );
        assert!(app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
    }

    #[test]
    fn closing_a_panel_keeps_it_and_the_toolbar_reopens_it() {
        let mut app = test_app();
        press(&mut app);
        let tree = panel(&mut app, InspectorOverlayPanel::Tree);
        let button = toggle_button(&mut app, InspectorOverlayPanel::Tree);
        app.world_mut().trigger(RequestClose { source: tree });
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(tree),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(button),
            Some(&ButtonVariant::Normal)
        );

        click(&mut app, button);
        assert_ne!(
            app.world().get::<Visibility>(tree),
            Some(&Visibility::Hidden)
        );
    }

    #[test]
    fn hiding_the_overlay_keeps_closed_panels_closed() {
        let mut app = test_app();
        press(&mut app);
        let button = toggle_button(&mut app, InspectorOverlayPanel::Tree);
        click(&mut app, button);
        press(&mut app);
        press(&mut app);
        assert!(!app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
        assert!(app
            .world_mut()
            .run_system_once(details_panel_visible)
            .unwrap());
    }

    #[test]
    fn closing_the_tree_stops_it_syncing_and_reopening_catches_up() {
        let mut app = test_app();
        press(&mut app);
        let button = toggle_button(&mut app, InspectorOverlayPanel::Tree);
        click(&mut app, button);
        let subject = app.world_mut().spawn(Name::new("Late")).id();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        assert_eq!(row_count(&mut app, subject), 0);
        click(&mut app, button);
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
        assert!(!world.run_system_once(entity_tree_visible).unwrap());

        world.entity_mut(root).insert(Visibility::Inherited);
        assert!(world.run_system_once(entity_tree_visible).unwrap());

        world.entity_mut(panel).insert(Visibility::Hidden);
        assert!(!world.run_system_once(entity_tree_visible).unwrap());
    }
}
