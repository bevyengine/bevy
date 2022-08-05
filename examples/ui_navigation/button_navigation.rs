use bevy::{prelude::*, ui_navigation::NavRequestSystem};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        // In order to get same-frame UI updates (so that it feels snappy),
        // make sure to update the visuals after the navigation system.
        .add_system(button_color.after(NavRequestSystem))
        .add_system(press_color.after(NavRequestSystem))
        .add_system(print_nav_events.after(NavRequestSystem))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const FOCUSED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn print_nav_events(mut events: EventReader<NavEvent>) {
    for event in events.iter() {
        println!("{:?}", event);
    }
}

fn press_color(
    mut events: EventReader<NavEvent>,
    mut interaction_query: Query<(&mut UiColor, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    for activated in events.nav_iter().activated() {
        if let Ok((mut color, children)) = interaction_query.get_mut(activated) {
            *color = PRESSED_BUTTON.into();
            let mut text = text_query.get_mut(children[0]).unwrap();
            text.sections[0].value = "Activated!".to_string();
        }
    }
}

// NOTE: We rely on this system running on focus update (Changed<Focusable>)
// to clear the "Activated!" text when focus changes
fn button_color(
    mut interaction_query: Query<
        (&Focusable, &mut UiColor, &Children),
        (Changed<Focusable>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (focus, mut color, children) in &mut interaction_query {
        let (new_color, new_text) = match focus.state() {
            FocusState::Focused => (FOCUSED_BUTTON, "Focused"),
            _ => (NORMAL_BUTTON, "Button"),
        };
        let mut text = text_query.get_mut(children[0]).unwrap();
        text.sections[0].value = new_text.to_string();
        *color = new_color.into();
    }
}

const BUTTON_WIDTH: f32 = 160.0;
const BUTTON_HEIGHT: f32 = 65.0;
const BUTTON_MARGIN: f32 = 10.0;
const MENU_HEIGHT: f32 = BUTTON_HEIGHT * 2.0 + BUTTON_MARGIN * 5.0;
const MENU_WIDTH: f32 = BUTTON_WIDTH * 2.0 + BUTTON_MARGIN * 5.0;

fn spawn_button(commands: &mut ChildBuilder, font: Handle<Font>) {
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(BUTTON_WIDTH), Val::Px(BUTTON_HEIGHT)),
                margin: UiRect::all(Val::Px(BUTTON_MARGIN)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            color: NORMAL_BUTTON.into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                "Button",
                TextStyle {
                    font,
                    font_size: 40.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            ));
        });
}
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // At least one camera must exist in bevy for UI to show up
    // (any sort will do, this works with Camera3dBundle as well)
    commands.spawn_bundle(Camera2dBundle::default());

    commands
        .spawn_bundle(NodeBundle {
            color: Color::NONE.into(),
            style: Style {
                size: Size::new(Val::Px(MENU_WIDTH), Val::Px(MENU_HEIGHT)),
                margin: UiRect::all(Val::Auto),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            for _ in 0..4 {
                spawn_button(commands, font.clone());
            }
        });
}
