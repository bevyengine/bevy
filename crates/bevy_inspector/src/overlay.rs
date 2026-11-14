//! A hidden inspector that opens over any app when a key is pressed.
//!
//! [`InspectorOverlayPlugin`] is part of `DefaultPlugins` when the `inspector` cargo feature is
//! enabled, so the app needs no code of its own. Its panels are not spawned until they are first
//! shown, and they render through the app's default UI camera, so a camera with a viewport limits
//! them to that viewport. A draggable toolbar shows and hides each panel, and the panels are
//! floating dialogs, so they can also be dragged and closed.

use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Update};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
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
    percent, px, AlignItems, BorderRadius, DefaultUiCamera, FlexDirection, GlobalZIndex, Node,
    PositionType, UiRect, UiSystems, Val,
};
use bevy_ui_widgets::{Activate, Dialog, DialogDragHandle, RequestClose};
use bevy_window::SystemCursorIcon;

use crate::{
    details_panel::{details_body, DetailsCollapsed},
    entity_tree::InspectorUi,
    picking::{pick_blocker, InspectorPickBlocker},
    shown,
    world_tabs::world_tabs,
    InspectorMode, InspectorPlugin, InspectorSource, InspectorSources,
};

/// Adds the inspector to an app as an overlay, hidden until [`toggle_key`](Self::toggle_key) is
/// pressed.
///
/// The inspector only walks the world while a view of it is showing.
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
            .add_systems(Update, (sync_panel_toggles, sync_mode_toggles))
            .add_systems(
                PostUpdate,
                sync_pick_blocker
                    .ambiguous_with_all()
                    .before(UiSystems::Prepare),
            );
        app.world_mut()
            .resource_mut::<DetailsCollapsed>()
            .by_default = true;
    }
}

#[derive(Resource)]
struct InspectorOverlayKey(KeyCode);

const OVERLAY_Z: i32 = 40;
const BLOCKER_Z: i32 = OVERLAY_Z - 1;

fn sync_pick_blocker(
    mode: Res<InspectorMode>,
    sources: Res<InspectorSources>,
    overlays: Query<Entity, With<InspectorOverlay>>,
    blockers: Query<Entity, With<InspectorPickBlocker>>,
    visibilities: Query<&Visibility>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let wanted = *mode == InspectorMode::Edit
        && sources.sources.contains(&InspectorSource::Local)
        && overlays
            .iter()
            .any(|overlay| shown(overlay, &visibilities, &nodes, &parents));
    let mut existing = blockers.iter();
    if wanted && existing.next().is_none() {
        commands.spawn(pick_blocker(BLOCKER_Z));
    }
    for blocker in if wanted { existing } else { blockers.iter() } {
        commands.entity(blocker).try_despawn();
    }
}

/// Marker for the root entity of the inspector overlay.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct InspectorOverlay;

/// A panel of the inspector overlay, shown and hidden independently from the toolbar.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorOverlayPanel {
    /// The tabbed views of the inspected world.
    #[default]
    Overview,
    /// The details of the selected entity.
    Details,
}

/// A toolbar button that shows and hides a panel.
#[derive(Component, Clone, Copy, Debug, Default)]
struct PanelToggle(InspectorOverlayPanel);

#[derive(Component, Clone, Copy, Debug, Default)]
struct ModeToggle(InspectorMode);

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

fn on_mode_toggle(
    activate: On<Activate>,
    toggles: Query<&ModeToggle>,
    mut mode: ResMut<InspectorMode>,
) {
    if let Ok(toggle) = toggles.get(activate.event_target()) {
        mode.set_if_neq(toggle.0);
    }
}

fn sync_mode_toggles(
    mode: Res<InspectorMode>,
    mut toggles: Query<(&ModeToggle, &mut ButtonVariant)>,
) {
    for (toggle, mut variant) in &mut toggles {
        variant.set_if_neq(ButtonVariant::selected(toggle.0 == *mode));
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
        GlobalZIndex(OVERLAY_Z)
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
            InspectorOverlayPanel::Overview
            Children [
                @panel_header("Overview")
                --
                @world_tabs()
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
                @caption: bsn! { @caption("Overview") },
                @variant: {ButtonVariant::Primary},
            }
            PanelToggle(InspectorOverlayPanel::Overview)
            on(on_panel_toggle)
            --
            @FeathersToolButton {
                @caption: bsn! { @caption("Details") },
                @variant: {ButtonVariant::Primary},
            }
            PanelToggle(InspectorOverlayPanel::Details)
            on(on_panel_toggle)
            --
            @FeathersToolButton {
                @caption: bsn! { @caption("Edit") },
            }
            ModeToggle(InspectorMode::Edit)
            on(on_mode_toggle)
            --
            @FeathersToolButton {
                @caption: bsn! { @caption("Preview") },
            }
            ModeToggle(InspectorMode::Preview)
            on(on_mode_toggle)
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
        entity_tree::{EntityTreeSync, InspectorEntityRow, InspectorHeaderRow, InspectorTreeView},
        entity_tree_visible,
        resource_list::{InspectorResourceList, InspectorResourceRow, ResourceListSync},
        resource_list_visible,
        world_tabs::{InspectorPage, InspectorPageTab},
        InspectableEntity, InspectorSelection, InspectorSource,
    };
    use bevy_app::TaskPoolPlugin;
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::{
        entity::Entity, hierarchy::ChildOf, name::Name, prelude::*, system::RunSystemOnce,
    };
    use bevy_feathers::theme::UiTheme;
    use bevy_ui_widgets::{SelectedTreeItem, ValueChange};

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
            (
                toggle_inspector_overlay,
                sync_panel_toggles,
                sync_mode_toggles,
            )
                .chain(),
        );
        app.add_systems(PostUpdate, sync_pick_blocker);
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
            .query::<&InspectorEntityRow>()
            .iter(world)
            .filter(|row| row.inspected == source)
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
        let tree = panel(&mut app, InspectorOverlayPanel::Overview);
        let details = panel(&mut app, InspectorOverlayPanel::Details);
        let button = toggle_button(&mut app, InspectorOverlayPanel::Overview);
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
        let tree = panel(&mut app, InspectorOverlayPanel::Overview);
        let button = toggle_button(&mut app, InspectorOverlayPanel::Overview);
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
        let button = toggle_button(&mut app, InspectorOverlayPanel::Overview);
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
        let button = toggle_button(&mut app, InspectorOverlayPanel::Overview);
        click(&mut app, button);
        let subject = app.world_mut().spawn(Name::new("Late")).id();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        assert_eq!(row_count(&mut app, subject), 0);
        click(&mut app, button);
        assert_eq!(row_count(&mut app, subject), 1);
    }

    fn select_page(app: &mut App, page: InspectorPage) {
        let world = app.world_mut();
        let (tab, strip) = world
            .query::<(Entity, &InspectorPageTab, &ChildOf)>()
            .iter(world)
            .find(|(_, tab, _)| tab.0 == page)
            .map(|(tab, _, strip)| (tab, strip.parent()))
            .unwrap();
        world.trigger(ValueChange {
            source: strip,
            value: Some(tab),
            is_final: true,
        });
        app.update();
    }

    #[test]
    fn tabs_switch_between_the_entity_tree_and_the_resource_list() {
        let mut app = test_app();
        press(&mut app);
        assert!(app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
        assert!(!app
            .world_mut()
            .run_system_once(resource_list_visible)
            .unwrap());

        select_page(&mut app, InspectorPage::Resources);
        assert!(!app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
        assert!(app
            .world_mut()
            .run_system_once(resource_list_visible)
            .unwrap());
        let world = app.world_mut();
        assert!(world
            .query::<&InspectorResourceRow>()
            .iter(world)
            .next()
            .is_some());

        select_page(&mut app, InspectorPage::Entities);
        assert!(app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
        assert!(!app
            .world_mut()
            .run_system_once(resource_list_visible)
            .unwrap());
    }

    #[test]
    fn selecting_a_resource_clears_the_entity_tree_highlight() {
        let mut app = test_app();
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        app.insert_resource(Marker);
        press(&mut app);
        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(subject, InspectorSource::Local));
        app.update();
        let world = app.world_mut();
        let tree = world
            .query_filtered::<Entity, With<InspectorTreeView>>()
            .single(world)
            .unwrap();
        assert!(app
            .world()
            .get::<SelectedTreeItem>(tree)
            .unwrap()
            .0
            .is_some());

        select_page(&mut app, InspectorPage::Resources);
        let list = single::<With<InspectorResourceList>>(&mut app);
        let row = resource_row(&mut app).unwrap();
        select_row(&mut app, list, row);
        assert_eq!(app.world().get::<SelectedTreeItem>(tree).unwrap().0, None);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(list).unwrap().0,
            Some(row)
        );
    }

    #[derive(Resource)]
    struct Marker;

    fn single<F: bevy_ecs::query::QueryFilter>(app: &mut App) -> Entity {
        let world = app.world_mut();
        world.query_filtered::<Entity, F>().single(world).unwrap()
    }

    fn resource_row(app: &mut App) -> Option<Entity> {
        let world = app.world_mut();
        let id = world.component_id::<Marker>()?;
        let inspected = world.resource_entities().get(id)?;
        world
            .query::<(Entity, &InspectorResourceRow)>()
            .iter(world)
            .find_map(|(row, shown)| (shown.inspected == inspected).then_some(row))
    }

    fn sync_resources(app: &mut App) {
        app.world_mut()
            .resource_mut::<ResourceListSync>()
            .set_dirty();
        app.update();
    }

    fn select_row(app: &mut App, list: Entity, row: Entity) {
        app.world_mut().trigger(ValueChange {
            source: list,
            value: Some(row),
            is_final: true,
        });
        app.update();
    }

    #[test]
    fn a_selected_header_stays_highlighted_until_a_resource_is_selected() {
        let mut app = test_app();
        app.insert_resource(Marker);
        press(&mut app);
        let tree = single::<With<InspectorTreeView>>(&mut app);
        let header = single::<With<InspectorHeaderRow>>(&mut app);
        select_row(&mut app, tree, header);
        assert_eq!(app.world().resource::<InspectorSelection>().0, None);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(tree).unwrap().0,
            Some(header)
        );

        select_page(&mut app, InspectorPage::Resources);
        let list = single::<With<InspectorResourceList>>(&mut app);
        let row = resource_row(&mut app).unwrap();
        select_row(&mut app, list, row);
        assert_eq!(app.world().get::<SelectedTreeItem>(tree).unwrap().0, None);
    }

    #[test]
    fn a_reinserted_resource_keeps_its_highlight() {
        let mut app = test_app();
        app.insert_resource(Marker);
        press(&mut app);
        select_page(&mut app, InspectorPage::Resources);
        let list = single::<With<InspectorResourceList>>(&mut app);
        let row = resource_row(&mut app).unwrap();
        select_row(&mut app, list, row);

        app.world_mut().remove_resource::<Marker>();
        sync_resources(&mut app);
        assert!(resource_row(&mut app).is_none());

        app.insert_resource(Marker);
        sync_resources(&mut app);
        let respawned = resource_row(&mut app).unwrap();
        assert_ne!(respawned, row);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(list).unwrap().0,
            Some(respawned)
        );
    }

    #[test]
    fn a_hidden_resource_tab_does_not_sync_and_catches_up_when_shown() {
        let mut app = test_app();
        press(&mut app);
        select_page(&mut app, InspectorPage::Resources);
        select_page(&mut app, InspectorPage::Entities);
        app.insert_resource(Marker);
        sync_resources(&mut app);
        assert!(resource_row(&mut app).is_none());

        select_page(&mut app, InspectorPage::Resources);
        assert!(resource_row(&mut app).is_some());
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

    fn mode_button(app: &mut App, mode: InspectorMode) -> Entity {
        let world = app.world_mut();
        world
            .query::<(Entity, &ModeToggle)>()
            .iter(world)
            .find(|(_, toggle)| toggle.0 == mode)
            .unwrap()
            .0
    }

    #[test]
    fn the_toolbar_switches_and_shows_the_mode() {
        let mut app = test_app();
        press(&mut app);
        let edit = mode_button(&mut app, InspectorMode::Edit);
        let preview = mode_button(&mut app, InspectorMode::Preview);
        assert_eq!(
            app.world().get::<ButtonVariant>(edit),
            Some(&ButtonVariant::Primary)
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(preview),
            Some(&ButtonVariant::Normal)
        );

        click(&mut app, preview);
        assert_eq!(
            *app.world().resource::<InspectorMode>(),
            InspectorMode::Preview
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(preview),
            Some(&ButtonVariant::Primary)
        );

        *app.world_mut().resource_mut::<InspectorMode>() = InspectorMode::Edit;
        app.update();
        assert_eq!(
            app.world().get::<ButtonVariant>(edit),
            Some(&ButtonVariant::Primary)
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(preview),
            Some(&ButtonVariant::Normal)
        );
    }

    fn blockers(app: &mut App) -> Vec<(Entity, bool, i32)> {
        let world = app.world_mut();
        world
            .query_filtered::<(Entity, Has<ChildOf>, &GlobalZIndex), With<InspectorPickBlocker>>()
            .iter(world)
            .map(|(blocker, parented, z)| (blocker, parented, z.0))
            .collect()
    }

    fn set_mode(app: &mut App, mode: InspectorMode) {
        *app.world_mut().resource_mut::<InspectorMode>() = mode;
        app.update();
    }

    #[test]
    fn the_pick_blocker_is_a_root_below_the_overlay_while_editing() {
        let mut app = test_app();
        app.update();
        assert!(blockers(&mut app).is_empty());
        press(&mut app);
        app.update();
        let overlay = overlays(&mut app)[0];
        let [(blocker, parented, z)] = blockers(&mut app)[..] else {
            panic!("expected one blocker");
        };
        assert!(!parented);
        let overlay_z = app.world().get::<GlobalZIndex>(overlay).unwrap().0;
        assert!(z < overlay_z && overlay_z < 50);

        select_page(&mut app, InspectorPage::Resources);
        app.update();
        assert_eq!(blockers(&mut app).len(), 1);

        set_mode(&mut app, InspectorMode::Preview);
        assert!(blockers(&mut app).is_empty());
        set_mode(&mut app, InspectorMode::Edit);
        assert_eq!(blockers(&mut app).len(), 1);

        press(&mut app);
        app.update();
        assert!(blockers(&mut app).is_empty());
        assert!(app.world().get_entity(blocker).is_err());
    }

    #[test]
    fn picking_from_the_resources_tab_selects_and_shows_the_entity_tree() {
        use bevy_picking::{
            backend::{HitData, PointerHits},
            pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
        };

        let mut app = test_app();
        press(&mut app);
        select_page(&mut app, InspectorPage::Resources);
        assert!(!app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
        let root = app.world_mut().spawn(Name::new("Root")).id();
        let subject = app
            .world_mut()
            .spawn((Name::new("Subject"), ChildOf(root)))
            .id();
        let camera = app.world_mut().spawn_empty().id();
        for action in [
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            app.world_mut().write_message(PointerHits::new(
                PointerId::Mouse,
                alloc::vec![(subject, HitData::new(camera, 1.0, None, None))],
                0.0,
            ));
            app.world_mut().write_message(PointerInput::new(
                PointerId::Mouse,
                Location {
                    target: bevy_camera::NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: bevy_math::Vec2::ZERO,
                },
                action,
            ));
            app.update();
        }
        assert_eq!(
            app.world().resource::<InspectorSelection>().0,
            Some(InspectableEntity::new(subject, InspectorSource::Local))
        );
        app.update();
        assert!(app
            .world_mut()
            .run_system_once(entity_tree_visible)
            .unwrap());
        app.update();
        let tree = single::<With<InspectorTreeView>>(&mut app);
        let row = app
            .world()
            .resource::<crate::entity_tree::TreeRowIndex>()
            .row(subject, InspectorSource::Local);
        assert!(row.is_some());
        assert_eq!(app.world().get::<SelectedTreeItem>(tree).unwrap().0, row);
    }
}
