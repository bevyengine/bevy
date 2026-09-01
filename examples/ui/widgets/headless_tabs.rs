//! Demonstrates the behavior-only tab widgets in `bevy_ui_widgets`.

use bevy::{
    ecs::template::{EntityTemplate, OptionTemplate},
    input_focus::{
        tab_navigation::{TabGroup, TabNavigationPlugin},
        InputFocus, InputFocusVisible,
    },
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Selected},
    ui_widgets::{
        tablist_self_update, ControlOrientation, SelectedTab, Tab, TabActivation, TabList,
        ValueChange,
    },
};

#[derive(Component, Default, Clone)]
struct ShowcaseTab;

#[derive(Resource, Default)]
struct ControlledSelection(Option<Entity>);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TabNavigationPlugin))
        .init_resource::<ControlledSelection>()
        .add_systems(Startup, showcase.spawn())
        .add_systems(Update, update_tab_styles)
        .run();
}

fn showcase() -> impl SceneList {
    bsn_list![
        Camera2d
        ---
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(24)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            row_gap: px(18),
            overflow: Overflow::scroll_y(),
        }
        BackgroundColor(Color::srgb(0.06, 0.07, 0.09))
        TabGroup
        Children [
            @section_label("Horizontal automatic - self-updating")
            ---
            #automatic
            @tab_strip(ControlOrientation::Horizontal)
            TabList {
                orientation: ControlOrientation::Horizontal,
                activation: TabActivation::Automatic,
            }
            @selected_tab(#automatic_general)
            on(tablist_self_update)
            Children [
                #automatic_general
                @tab_header("General")
                ---
                @tab_header("Rendering")
                ---
                @tab_header("Disabled")
                InteractionDisabled
            ]
            ---
            @section_label("Horizontal manual - focus and selection are separate")
            ---
            @tab_strip(ControlOrientation::Horizontal)
            TabList::default()
            @selected_tab(#manual_scene)
            on(tablist_self_update)
            Children [
                #manual_scene
                @tab_header("Scene")
                ---
                @tab_header("Assets")
                ---
                @tab_header("Inspector")
            ]
            ---
            @section_label("Vertical manual")
            ---
            @tab_strip(ControlOrientation::Vertical)
            TabList {
                orientation: ControlOrientation::Vertical,
                activation: TabActivation::Manual,
            }
            @selected_tab(#vertical_transform)
            on(tablist_self_update)
            Children [
                #vertical_transform
                @tab_header("Transform")
                ---
                @tab_header("Visibility")
                ---
                @tab_header("Metadata")
            ]
            ---
            @section_label("Controlled - observer updates external state")
            ---
            @tab_strip(ControlOrientation::Horizontal)
            TabList::default()
            @selected_tab(#controlled_a)
            on(controlled_selection)
            Children [
                #controlled_a
                @tab_header("External A")
                ---
                @tab_header("External B")
            ]
        ]
    ]
}

fn tab_strip(orientation: ControlOrientation) -> impl Scene {
    let flex_direction = match orientation {
        ControlOrientation::Horizontal => FlexDirection::Row,
        ControlOrientation::Vertical => FlexDirection::Column,
    };
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction,
            align_items: AlignItems::Stretch,
        }
        BackgroundColor(Color::srgb(0.10, 0.11, 0.14))
    }
}

fn tab_header(label: &'static str) -> impl Scene {
    bsn! {
        ShowcaseTab
        Tab
        Hovered
        Node {
            min_width: px(112),
            min_height: px(36),
            padding: UiRect::axes(px(12), px(8)),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BackgroundColor(Color::srgb(0.13, 0.14, 0.18))
        BorderColor::all(Color::srgb(0.24, 0.25, 0.30))
        Children [
            Text(label)
            TextFont {
                font_size: FontSize::Px(16.0)
            }
            TextColor(Color::srgb(0.88, 0.89, 0.92))
        ]
    }
}

fn selected_tab(tab: EntityTemplate) -> impl Scene {
    let tab = OptionTemplate::Some(tab);
    bsn! {
        SelectedTab({tab})
    }
}

fn section_label(label: &'static str) -> impl Scene {
    bsn! {
        Text(label)
        TextFont {
            font_size: FontSize::Px(17.0)
        }
        TextColor(Color::srgb(0.72, 0.76, 0.84))
    }
}

fn controlled_selection(
    change: On<ValueChange<Option<Entity>>>,
    mut state: ResMut<ControlledSelection>,
    mut commands: Commands,
) {
    state.0 = change.value;
    commands.entity(change.source).insert(SelectedTab(state.0));
}

fn update_tab_styles(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    mut tabs: Query<
        (
            Entity,
            &Hovered,
            Has<Selected>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ShowcaseTab>,
    >,
) {
    for (entity, hovered, selected, disabled, mut background, mut border) in &mut tabs {
        background.0 = match (disabled, selected, hovered.get()) {
            (true, _, _) => Color::srgb(0.10, 0.10, 0.11),
            (false, true, _) => Color::srgb(0.18, 0.34, 0.52),
            (false, false, true) => Color::srgb(0.20, 0.21, 0.26),
            _ => Color::srgb(0.13, 0.14, 0.18),
        };
        border.set_all(if focus_visible.0 && focus.get() == Some(entity) {
            Color::srgb(0.45, 0.78, 1.0)
        } else {
            Color::srgb(0.24, 0.25, 0.30)
        });
    }
}
