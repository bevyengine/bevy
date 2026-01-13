//! Overlay showing diagnostics
//!
//! The window can be created using the [`DiagnosticsOverlay`] component

use alloc::borrow::Cow;
use core::time::Duration;

use bevy_app::prelude::*;
use bevy_color::{palettes, prelude::*};
use bevy_diagnostic::{DiagnosticPath, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{prelude::*, relationship::Relationship};
use bevy_pbr::{diagnostic::MaterialAllocatorDiagnosticPlugin, StandardMaterial};
use bevy_picking::prelude::*;
use bevy_render::diagnostic::MeshAllocatorDiagnosticPlugin;
use bevy_text::prelude::*;
use bevy_time::common_conditions::on_timer;
use bevy_ui::prelude::*;
use tracing::error;

/// Initial offset from the top left corner of the window
/// for the diagnostics overlay
const INITIAL_OFFSET: Val = Val::Px(32.);
/// Alpha value for [`BackgroundColor`] of the overlay
const BACKGROUND_COLOR_ALPHA: f32 = 0.75;
/// Row and column gap for the diagnostics overlay
const ROW_COLUMN_GAP: Val = Val::Px(4.);
/// Padding for cels of the diagnostics overlay
const DEFAULT_PADDING: UiRect = UiRect::all(Val::Px(4.));

/// Diagnostics overlay
///
/// Spawning an entity with this component will create a draggable and collapsable window
/// that presents the diagnostics passed to the constructor
///
/// If any value is showing as missing, means that no value was stored to the [`DiagnosticPath`],
/// so make sure that the plugin that writes to it is properly set up.
#[derive(Component)]
pub struct DiagnosticsOverlay {
    title: Cow<'static, str>,
    diagnostic_paths: Vec<DiagnosticPath>,
}

impl DiagnosticsOverlay {
    /// Creates a new instance of a [`DiagnosticsOverlay`]
    pub fn new(title: impl Into<Cow<'static, str>>, diagnostic_paths: Vec<DiagnosticPath>) -> Self {
        Self {
            title: title.into(),
            diagnostic_paths,
        }
    }

    /// Gets the title of the [`DiagnosticsOverlay`] window
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Gets the [`DiagnosticPath`] registered to this [`DiagnosticsOverlay`]
    pub fn diagnostic_paths(&self) -> &[DiagnosticPath] {
        &self.diagnostic_paths
    }

    /// Create a [`DiagnosticsOverlay`] with the diagnostcs from [`FrameTimeDiagnosticsPlugin`]
    pub fn fps() -> Self {
        Self {
            title: Cow::Owned("Fps".to_owned()),
            diagnostic_paths: vec![
                FrameTimeDiagnosticsPlugin::FPS,
                FrameTimeDiagnosticsPlugin::FRAME_TIME,
                FrameTimeDiagnosticsPlugin::FRAME_COUNT,
            ],
        }
    }

    /// Create a [`DiagnosticsOverlay`] with the diagnostics from
    /// [`MaterialAllocatorDiagnosticPlugin`] of [`StandardMaterial`] and
    /// [`MeshAllocatorDiagnosticPlugin`]
    pub fn mesh_and_standard_material() -> Self {
        Self {
            title: Cow::Owned("Mesh and standard materials".to_owned()),
            diagnostic_paths: vec![
                MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::slabs_diagnostic_path(),
                MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::slabs_size_diagnostic_path(),
                MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::allocations_diagnostic_path(
                ),
                MeshAllocatorDiagnosticPlugin::slabs_diagnostic_path().clone(),
                MeshAllocatorDiagnosticPlugin::slabs_size_diagnostic_path().clone(),
                MeshAllocatorDiagnosticPlugin::allocations_diagnostic_path().clone(),
            ],
        }
    }
}

/// Plugin that builds a visual overlay to present diagnostics
pub struct DiagnosticsOverlayPlugin;

impl Plugin for DiagnosticsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            rebuild_diagnostics_list.run_if(on_timer(Duration::from_secs(1))),
        );

        app.add_observer(build_overlay);
        app.add_observer(drag_by_header);
        app.add_observer(collapse_on_click_to_header);
    }
}

/// Header of the overlay
#[derive(Component)]
struct DiagnosticOverlayHeader;

/// Section of the overlay that will have the diagnostics
#[derive(Component)]
struct DiagnosticsList;

fn rebuild_diagnostics_list(
    mut commands: Commands,
    diagnostics_overlays: Query<&DiagnosticsOverlay>,
    diagnostics_lists: Query<(Entity, &ChildOf), With<DiagnosticsList>>,
    diagnostics: Res<DiagnosticsStore>,
) {
    for (entity, child_of) in diagnostics_lists {
        commands.entity(entity).despawn_children();

        let Ok(diagnostics_overlay) = diagnostics_overlays.get(child_of.get()) else {
            error!("Failed to get list of diagnostics path to display on the overlay.");
            continue;
        };

        for (i, diagnostic_path) in diagnostics_overlay.diagnostic_paths.iter().enumerate() {
            let maybe_diagnostic = diagnostics.get(diagnostic_path);
            let diagnostic = maybe_diagnostic
                .map(|diagnostic| {
                    format!("{}{}", diagnostic.value().unwrap_or(0.), diagnostic.suffix)
                })
                .unwrap_or("Missing".to_owned());

            commands.spawn((
                ChildOf(entity),
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
                ChildOf(entity),
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
}

fn build_overlay(
    event: On<Add, DiagnosticsOverlay>,
    mut commands: Commands,
    diagnostics_overlays: Query<&DiagnosticsOverlay>,
) {
    let entity = event.entity;
    let Ok(diagnostics_overlay) = diagnostics_overlays.get(entity) else {
        unreachable!("DiagnosticsOverlay must be available.");
    };

    commands.entity(entity).insert((
        Node {
            top: INITIAL_OFFSET,
            left: INITIAL_OFFSET,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
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
                    Text::new(diagnostics_overlay.title.as_ref()),
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

fn drag_by_header(
    mut event: On<Pointer<Drag>>,
    mut overlay: Query<&mut Node, With<DiagnosticsOverlay>>,
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
    mut overlay: Query<&Children, With<DiagnosticsOverlay>>,
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
