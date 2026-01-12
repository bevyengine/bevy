//! Overlay showing some of the render assets diagnostics
//!
//! The overlay will contain information about the mesh slabs and allocations
//! and [`StandardMaterial`]

use core::time::Duration;

use bevy_app::prelude::*;
use bevy_color::{palettes, prelude::*};
use bevy_diagnostic::DiagnosticsStore;
use bevy_ecs::{prelude::*, relationship::Relationship};
use bevy_pbr::{diagnostic::MaterialAllocatorDiagnosticPlugin, StandardMaterial};
use bevy_picking::prelude::*;
use bevy_render::diagnostic::MeshAllocatorDiagnosticPlugin;
use bevy_text::prelude::*;
use bevy_time::common_conditions::on_timer;
use bevy_ui::prelude::*;

/// Initial offset from the top left corner of the window
/// for the diagnostics overlay
const INITIAL_OFFSET: Val = Val::Px(32.);
/// Alpha value for [`BackgroundColor`] of the overlay
const BACKGROUND_COLOR_ALPHA: f32 = 0.75;
/// Row and column gap for the diagnostics overlay
const ROW_COLUMN_GAP: Val = Val::Px(4.);
/// Padding for cels of the diagnostics overlay
const DEFAULT_PADDING: UiRect = UiRect::all(Val::Px(4.));

/// Plugin that builds a visual overlay to present
/// render assets diagnostics
pub struct RenderAssetOverlayPlugin;

impl Plugin for RenderAssetOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            rebuild_diagnostics_list.run_if(on_timer(Duration::from_secs(1))),
        );

        app.add_observer(drag_by_header);
        app.add_observer(collapse_on_click_to_header);
    }
}

/// Root of the overlay
#[derive(Component)]
struct DiagnosticOverlay;

/// Header of the overlay
#[derive(Component)]
struct DiagnosticOverlayHeader;

/// Section of the overlay that will have the diagnostics
#[derive(Component)]
struct DiagnosticsList;

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            top: INITIAL_OFFSET,
            left: INITIAL_OFFSET,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        DiagnosticOverlay,
        Pickable::IGNORE,
        children![
            (
                Node {
                    padding: DEFAULT_PADDING,
                    ..Default::default()
                },
                DiagnosticOverlayHeader,
                BackgroundColor(
                    palettes::tailwind::GRAY_900
                        .with_alpha(BACKGROUND_COLOR_ALPHA)
                        .into()
                ),
                children![(
                    Text::new("Render assets"),
                    TextFont {
                        font_size: 12.,
                        ..Default::default()
                    },
                    Pickable::IGNORE
                )],
            ),
            (
                Node {
                    display: Display::Grid,
                    row_gap: ROW_COLUMN_GAP,
                    column_gap: ROW_COLUMN_GAP,
                    padding: DEFAULT_PADDING,
                    ..Default::default()
                },
                DiagnosticsList,
                BackgroundColor(
                    palettes::tailwind::GRAY_600
                        .with_alpha(BACKGROUND_COLOR_ALPHA)
                        .into()
                ),
            )
        ],
    ));
}

fn rebuild_diagnostics_list(
    mut commands: Commands,
    diagnostics_list: Single<Entity, With<DiagnosticsList>>,
    diagnostics: Res<DiagnosticsStore>,
) {
    let diagnostic_paths = [
        MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::slabs_diagnostic_path(),
        MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::slabs_size_diagnostic_path(),
        MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::allocations_diagnostic_path(),
        MeshAllocatorDiagnosticPlugin::slabs_diagnostic_path().clone(),
        MeshAllocatorDiagnosticPlugin::slabs_size_diagnostic_path().clone(),
        MeshAllocatorDiagnosticPlugin::allocations_diagnostic_path().clone(),
    ];

    commands.entity(*diagnostics_list).despawn_children();

    for (i, diagnostic_path) in diagnostic_paths.into_iter().enumerate() {
        let maybe_diagnostic = diagnostics.get(&diagnostic_path);
        let diagnostic = maybe_diagnostic
            .map(|diagnostic| format!("{}{}", diagnostic.value().unwrap_or(0.), diagnostic.suffix))
            .unwrap_or("Missing".to_owned());

        commands.spawn((
            ChildOf(*diagnostics_list),
            Node {
                grid_row: GridPlacement::start(i as i16 + 1),
                grid_column: GridPlacement::start(1),
                ..Default::default()
            },
            Pickable::IGNORE,
            children![(
                Text::new(diagnostic_path.to_string()),
                TextFont {
                    font_size: 10.,
                    ..Default::default()
                },
                Pickable::IGNORE,
            )],
        ));
        commands.spawn((
            ChildOf(*diagnostics_list),
            Node {
                grid_row: GridPlacement::start(i as i16 + 1),
                grid_column: GridPlacement::start(2),
                ..Default::default()
            },
            Pickable::IGNORE,
            children![(
                Text::new(diagnostic),
                TextFont {
                    font_size: 10.,
                    ..Default::default()
                },
                Pickable::IGNORE,
            )],
        ));
    }
}

fn drag_by_header(
    mut event: On<Pointer<Drag>>,
    mut overlay: Query<&mut Node, With<DiagnosticOverlay>>,
    headers: Query<&ChildOf, With<DiagnosticOverlayHeader>>,
) {
    let entity = event.entity;
    if let Ok(child_of) = headers.get(entity) {
        event.propagate(false);
        let Ok(mut node) = overlay.get_mut(child_of.get()) else {
            unreachable!("Render asset diagnostic overlay hierarchy is malformed.");
        };
        let delta = event.delta;
        let Val::Px(top) = &mut node.top else {
            unreachable!("Node must have `top` using `Val::Px`.");
        };
        *top += delta.y;
        let Val::Px(left) = &mut node.left else {
            unreachable!("Node must have `left` using `Val::Px`.");
        };
        *left += delta.x;
    }
}

fn collapse_on_click_to_header(
    mut event: On<Pointer<Click>>,
    mut overlay: Query<&Children, With<DiagnosticOverlay>>,
    mut lists: Query<&mut Node, With<DiagnosticsList>>,
    headers: Query<&ChildOf, With<DiagnosticOverlayHeader>>,
) {
    if event.duration > Duration::from_millis(250) {
        return;
    }

    let entity = event.entity;
    if let Ok(child_of) = headers.get(entity) {
        event.propagate(false);

        let Ok(children) = overlay.get_mut(child_of.get()) else {
            unreachable!("Render asset diagnostic overlay hierarchy is malformed.");
        };
        let mut lists_iter = lists.iter_many_mut(children.collection());

        let Some(mut node) = lists_iter.fetch_next() else {
            unreachable!("Render asset diagnostic overlay must have a child with DiagnosticList.");
        };

        if node.display == Display::Grid {
            node.display = Display::None;
        } else if node.display == Display::None {
            node.display = Display::Grid;
        } else {
            unreachable!("Diagnostic list `Display` must be either `Grid` or `None`.");
        }
    }
}
