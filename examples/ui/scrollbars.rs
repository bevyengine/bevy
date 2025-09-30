//! Demonstrations of scrolling and scrollbars.

use bevy::{
    ecs::{relationship::RelatedSpawner, spawn::SpawnWith},
    input_focus::{
        tab_navigation::{TabGroup, TabNavigationPlugin},
        InputDispatchPlugin,
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        ControlOrientation, CoreScrollbarDragState, CoreScrollbarThumb, Scrollbar, ScrollbarPlugin,
    },
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ScrollbarPlugin,
            InputDispatchPlugin,
            TabNavigationPlugin,
        ))
        .insert_resource(UiScale(1.25))
        .add_systems(Startup, setup_view_root)
        .add_systems(Update, update_scrollbar_thumb)
        .run();
}

fn setup_view_root(mut commands: Commands) {
    let camera = commands.spawn((Camera::default(), Camera2d)).id();

    commands.spawn((
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            right: px(0),
            bottom: px(0),
            padding: UiRect::all(px(3)),
            row_gap: px(6),
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        UiTargetCamera(camera),
        TabGroup::default(),
        Children::spawn((Spawn(Text::new("Scrolling")), Spawn(scroll_area_demo()))),
    ));
}

/// Create a scrolling area.
///
/// The "scroll area" is a container that can be scrolled. It has a nested structure which is
/// three levels deep:
/// - The outermost node is a grid that contains the scroll area and the scrollbars.
/// - The scroll area is a flex container that contains the scrollable content. This
///   is the element that has the `overflow: scroll` property.
/// - The scrollable content consists of the elements actually displayed in the scrolling area.
fn scroll_area_demo() -> impl Bundle {
    (
        // Frame element which contains the scroll area and scrollbars.
        Node {
            display: Display::Grid,
            width: px(200),
            height: px(150),
            grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.), RepeatedGridTrack::auto(1)],
            grid_template_rows: vec![RepeatedGridTrack::flex(1, 1.), RepeatedGridTrack::auto(1)],
            row_gap: px(2),
            column_gap: px(2),
            ..default()
        },
        Children::spawn((SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
            // The actual scrolling area.
            // Note that we're using `SpawnWith` here because we need to get the entity id of the
            // scroll area in order to set the target of the scrollbars.
            let scroll_area_id = parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(4)),
                        overflow: Overflow::scroll(),
                        ..default()
                    },
                    BackgroundColor(colors::GRAY1.into()),
                    ScrollPosition(Vec2::new(0.0, 10.0)),
                    Children::spawn((
                        // The actual content of the scrolling area
                        Spawn(text_row("Alpha Wolf")),
                        Spawn(text_row("Beta Blocker")),
                        Spawn(text_row("Delta Sleep")),
                        Spawn(text_row("Gamma Ray")),
                        Spawn(text_row("Epsilon Eridani")),
                        Spawn(text_row("Zeta Function")),
                        Spawn(text_row("Lambda Calculus")),
                        Spawn(text_row("Nu Metal")),
                        Spawn(text_row("Pi Day")),
                        Spawn(text_row("Chi Pants")),
                        Spawn(text_row("Psi Powers")),
                        Spawn(text_row("Omega Fatty Acid")),
                    )),
                ))
                .id();

            // Vertical scrollbar
            parent.spawn((
                Node {
                    min_width: px(8),
                    grid_row: GridPlacement::start(1),
                    grid_column: GridPlacement::start(2),
                    ..default()
                },
                Scrollbar {
                    orientation: ControlOrientation::Vertical,
                    target: scroll_area_id,
                    min_thumb_length: 8.0,
                },
                Children::spawn(Spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Hovered::default(),
                    BackgroundColor(colors::GRAY2.into()),
                    BorderRadius::all(px(4)),
                    CoreScrollbarThumb,
                ))),
            ));

            // Horizontal scrollbar
            parent.spawn((
                Node {
                    min_height: px(8),
                    grid_row: GridPlacement::start(2),
                    grid_column: GridPlacement::start(1),
                    ..default()
                },
                Scrollbar {
                    orientation: ControlOrientation::Horizontal,
                    target: scroll_area_id,
                    min_thumb_length: 8.0,
                },
                Children::spawn(Spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Hovered::default(),
                    BackgroundColor(colors::GRAY2.into()),
                    BorderRadius::all(px(4)),
                    CoreScrollbarThumb,
                ))),
            ));
        }),)),
    )
}

/// Create a list row
fn text_row(caption: &str) -> impl Bundle {
    (
        Text::new(caption),
        TextFont {
            font_size: 14.0,
            ..default()
        },
    )
}

// Update the color of the scrollbar thumb.
fn update_scrollbar_thumb(
    mut q_thumb: Query<
        (&mut BackgroundColor, &Hovered, &CoreScrollbarDragState),
        (
            With<CoreScrollbarThumb>,
            Or<(Changed<Hovered>, Changed<CoreScrollbarDragState>)>,
        ),
    >,
) {
    for (mut thumb_bg, Hovered(is_hovering), drag) in q_thumb.iter_mut() {
        let color: Color = if *is_hovering || drag.dragging {
            // If hovering, use a lighter color
            colors::GRAY3
        } else {
            // Default color for the slider
            colors::GRAY2
        }
        .into();

        if thumb_bg.0 != color {
            // Update the color of the thumb
            thumb_bg.0 = color;
        }
    }
}

mod colors {
    use bevy::color::Srgba;

    pub const GRAY1: Srgba = Srgba::new(0.224, 0.224, 0.243, 1.0);
    pub const GRAY2: Srgba = Srgba::new(0.486, 0.486, 0.529, 1.0);
    pub const GRAY3: Srgba = Srgba::new(1.0, 1.0, 1.0, 1.0);
}
