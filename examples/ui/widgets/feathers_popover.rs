//! Demonstrates Feathers popover arrows across all placements and while scrolling.

use bevy::{
    feathers::{
        controls::{FeathersButton, FeathersScrollbar},
        dark_theme::create_dark_theme,
        display::{caption, popover::FeathersPopoverArrow},
        palette,
        theme::{ThemeBackgroundColor, UiTheme},
        tokens, FeathersPlugins,
    },
    prelude::*,
    ui_widgets::{
        popover::{
            Popover, PopoverAlign, PopoverHideWhenAnchorClipped, PopoverPlacement, PopoverShift,
            PopoverSide,
        },
        ControlOrientation, ScrollArea,
    },
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![
        Camera2d,
        (
            Node {
                width: percent(100),
                height: percent(100),
                padding: UiRect {
                    left: px(16),
                    right: px(16),
                    top: px(52),
                    bottom: px(16),
                },
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(12),
            }
            ThemeBackgroundColor(tokens::WINDOW_BG)
            Children [
                (
                    Node {
                        width: px(728),
                        height: px(248),
                        margin: UiRect::bottom(px(44)),
                        display: Display::Grid,
                        grid_template_columns: {
                            RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(5, 136.0)
                        },
                        grid_template_rows: {
                            RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(5, 40.0)
                        },
                        column_gap: px(12),
                        row_gap: px(12),
                    }
                    Children [
                        placement_anchor("TL", "Start", PopoverSide::Top, PopoverAlign::Start, 2, 1),
                        placement_anchor("Top", "Center", PopoverSide::Top, PopoverAlign::Center, 3, 1),
                        placement_anchor("TR", "End", PopoverSide::Top, PopoverAlign::End, 4, 1),
                        placement_anchor("LT", "Start", PopoverSide::Left, PopoverAlign::Start, 1, 2),
                        placement_anchor("RT", "Start", PopoverSide::Right, PopoverAlign::Start, 5, 2),
                        placement_anchor("Left", "Center", PopoverSide::Left, PopoverAlign::Center, 1, 3),
                        placement_anchor("Right", "Center", PopoverSide::Right, PopoverAlign::Center, 5, 3),
                        placement_anchor("LB", "End", PopoverSide::Left, PopoverAlign::End, 1, 4),
                        placement_anchor("RB", "End", PopoverSide::Right, PopoverAlign::End, 5, 4),
                        placement_anchor("BL", "Start", PopoverSide::Bottom, PopoverAlign::Start, 2, 5),
                        placement_anchor("Bottom", "Center", PopoverSide::Bottom, PopoverAlign::Center, 3, 5),
                        placement_anchor("BR", "End", PopoverSide::Bottom, PopoverAlign::End, 4, 5),
                    ]
                ),
                (
                    Node {
                        width: percent(100),
                        max_width: px(728),
                        height: px(180),
                        position_type: PositionType::Relative,
                    }
                    ThemeBackgroundColor(tokens::PANE_BODY_BG)
                    Children [
                        (
                            #scroll_area
                            Node {
                                width: percent(100),
                                height: percent(100),
                                padding: UiRect {
                                    left: px(12),
                                    right: px(24),
                                    top: px(12),
                                    bottom: px(24),
                                },
                                overflow: Overflow::scroll(),
                            }
                            ScrollArea
                            ScrollPosition(Vec2::new(180.0, 180.0))
                            Children [
                                (
                                    Node {
                                        min_width: px(1200),
                                        min_height: px(640),
                                        position_type: PositionType::Relative,
                                    }
                                    Children [
                                        (
                                            Node {
                                                width: px(136),
                                                height: px(48),
                                                position_type: PositionType::Absolute,
                                                left: px(540),
                                                top: px(290),
                                            }
                                            Children [
                                                (
                                                    @FeathersButton {
                                                        @caption: bsn! { caption("Scrollable anchor") },
                                                    }
                                                    Node {
                                                        width: percent(100),
                                                        height: percent(100),
                                                    }
                                                ),
                                                popover_with_positions(
                                                    "Tracks the anchor",
                                                    [
                                                        PopoverSide::Left,
                                                        PopoverSide::Right,
                                                        PopoverSide::Top,
                                                        PopoverSide::Bottom,
                                                    ]
                                                    .map(|side| PopoverPlacement {
                                                        side,
                                                        align: PopoverAlign::Center,
                                                        gap: 8.0,
                                                    })
                                                    .to_vec(),
                                                    PopoverAppearance::Glass,
                                                ),
                                            ]
                                        )
                                    ]
                                )
                            ]
                        ),
                        (
                            @FeathersScrollbar {
                                @target: #scroll_area,
                                @orientation: { ControlOrientation::Vertical },
                                @auto_hide: false,
                            }
                            Node {
                                position_type: PositionType::Absolute,
                                right: px(5),
                                top: px(8),
                                bottom: px(18),
                                width: px(6),
                            }
                        ),
                        (
                            @FeathersScrollbar {
                                @target: #scroll_area,
                                @orientation: { ControlOrientation::Horizontal },
                                @auto_hide: false,
                            }
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(8),
                                right: px(18),
                                bottom: px(5),
                                height: px(6),
                            }
                        ),
                    ]
                ),
            ]
        )
    ]
}

fn placement_anchor(
    button_text: &'static str,
    popover_text: &'static str,
    side: PopoverSide,
    align: PopoverAlign,
    column: i16,
    row: i16,
) -> impl Scene {
    let appearance = match (side, align) {
        (PopoverSide::Top, PopoverAlign::Center) => PopoverAppearance::Glass,
        (PopoverSide::Left, PopoverAlign::Center) => PopoverAppearance::Accent,
        (PopoverSide::Right, PopoverAlign::Center) => PopoverAppearance::Warning,
        (PopoverSide::Bottom, PopoverAlign::Center) => PopoverAppearance::Success,
        _ => PopoverAppearance::Menu,
    };
    bsn! {
        Node {
            width: px(136),
            height: px(40),
            grid_column: { GridPlacement::start(column) },
            grid_row: { GridPlacement::start(row) },
            justify_self: JustifySelf::Center,
            align_self: AlignSelf::Center,
        }
        Children [
            (
                @FeathersButton {
                    @caption: bsn! { caption(button_text) },
                }
                Node {
                    width: percent(100),
                    height: percent(100),
                }
            ),
            popover_label(
                popover_text,
                PopoverPlacement {
                    side,
                    align,
                    gap: 8.0,
                },
                appearance,
            ),
        ]
    }
}

fn popover_label(
    text: &'static str,
    placement: PopoverPlacement,
    appearance: PopoverAppearance,
) -> impl Scene {
    popover_with_positions(text, vec![placement], appearance)
}

fn popover_with_positions(
    text: &'static str,
    positions: Vec<PopoverPlacement>,
    appearance: PopoverAppearance,
) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(10), px(6)),
            border: px(appearance.border_width()),
            border_radius: px(4),
        }
        Pickable::IGNORE
        GlobalZIndex(10)
        BackgroundColor({ appearance.background() })
        BorderColor::all(appearance.border_color())
        BoxShadow::new(
            appearance.shadow_color(),
            px(0),
            px(1),
            px(1),
            px(5),
        )
        Popover {
            positions,
            window_margin: 4.0,
        }
        PopoverShift
        PopoverHideWhenAnchorClipped
        Children [
            caption(text),
            @FeathersPopoverArrow,
        ]
    }
}

/// Styles used to show that arrows use their parent popover's colors.
#[derive(Clone, Copy)]
enum PopoverAppearance {
    Menu,
    Glass,
    Accent,
    Warning,
    Success,
}

impl PopoverAppearance {
    fn background(self) -> Color {
        match self {
            Self::Menu => palette::GRAY_1,
            Self::Glass => palette::ACCENT.with_alpha(0.30),
            Self::Accent => Srgba::new(0.04, 0.30, 0.28, 0.88).into(),
            Self::Warning => Srgba::new(0.38, 0.20, 0.04, 0.92).into(),
            Self::Success => palette::Y_AXIS.with_alpha(0.78),
        }
    }

    fn border_color(self) -> Color {
        match self {
            Self::Menu => palette::WARM_GRAY_1,
            Self::Glass => palette::ACCENT.with_alpha(0.85),
            Self::Accent => palette::ACCENT,
            Self::Warning => Srgba::new(1.0, 0.67, 0.24, 1.0).into(),
            Self::Success => palette::LIGHT_GRAY_1.with_alpha(0.9),
        }
    }

    fn border_width(self) -> f32 {
        match self {
            Self::Glass | Self::Accent => 2.0,
            Self::Menu | Self::Warning | Self::Success => 1.0,
        }
    }

    fn shadow_color(self) -> Color {
        Color::BLACK.with_alpha(match self {
            Self::Glass => 0.25,
            Self::Menu | Self::Accent | Self::Warning | Self::Success => 0.55,
        })
    }
}
