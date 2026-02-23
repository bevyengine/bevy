use bevy::{
    camera_controller::free_camera::FreeCameraState,
    feathers::{
        self,
        controls::{button, checkbox, ButtonProps},
        theme::{ThemeBackgroundColor, ThemedText},
    },
    pbr::wireframe::WireframeConfig,
    prelude::*,
    ui::Checked,
    ui_widgets::{checkbox_self_update, observe, Activate, ValueChange},
};
use rand::RngExt;

use crate::assets::CityAssets;
use crate::generate_city::{spawn_city, CityRoot};

#[derive(Resource)]
pub struct Settings {
    pub simulate_cars: bool,
    pub shadow_maps_enabled: bool,
    pub contact_shadows_enabled: bool,
    pub wireframe_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            simulate_cars: true,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            wireframe_enabled: false,
        }
    }
}

pub fn setup_settings_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        ThemeBackgroundColor(feathers::tokens::WINDOW_BG),
        observe(
            |_: On<Pointer<Over>>, mut free_camera_state: Single<&mut FreeCameraState>| {
                free_camera_state.enabled = false;
            },
        ),
        observe(
            |_: On<Pointer<Out>>, mut free_camera_state: Single<&mut FreeCameraState>| {
                free_camera_state.enabled = true;
            },
        ),
        children![(
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                row_gap: px(8),
                ..default()
            },
            children![
                (Text("Settings".to_owned())),
                (
                    checkbox(Checked, Spawn((Text::new("Simulate Cars"), ThemedText))),
                    observe(checkbox_self_update),
                    observe(
                        |change: On<ValueChange<bool>>, mut settings: ResMut<Settings>| {
                            settings.simulate_cars = change.value;
                        }
                    )
                ),
                (
                    checkbox(
                        Checked,
                        Spawn((Text::new("Shadow maps enabled"), ThemedText))
                    ),
                    observe(checkbox_self_update),
                    observe(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut directional_lights: Query<&mut DirectionalLight>| {
                            settings.shadow_maps_enabled = change.value;
                            for mut light in &mut directional_lights {
                                light.shadow_maps_enabled = change.value;

                            }
                        }
                    )
                ),
                (
                    checkbox(
                        Checked,
                        Spawn((Text::new("Contact shadows enabled"), ThemedText))
                    ),
                    observe(checkbox_self_update),
                    observe(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut directional_lights: Query<&mut DirectionalLight>| {
                            settings.contact_shadows_enabled = change.value;
                            for mut light in &mut directional_lights {
                                light.contact_shadows_enabled = change.value;

                            }
                        }
                    )
                ),
                (
                    checkbox((), Spawn((Text::new("Wireframe Enabled"), ThemedText))),
                    observe(checkbox_self_update),
                    observe(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut wireframe_config: ResMut<WireframeConfig>| {
                            settings.wireframe_enabled = change.value;
                            wireframe_config.global = change.value;
                        }
                    )
                ),
                (
                    button(
                        ButtonProps::default(),
                        (),
                        Spawn((Text::new("Regenerate City"), ThemedText))
                    ),
                    observe(
                        |_activate: On<Activate>,
                         mut commands: Commands,
                         city_root: Single<Entity, With<CityRoot>>,
                         assets: Res<CityAssets>| {
                            commands.entity(*city_root).despawn();

                            let mut rng = rand::rng();
                            let seed = rng.random::<u64>();
                            println!("new seed: {seed}");
                            spawn_city(&mut commands, &assets, seed, 32);
                        }
                    )
                ),
            ]
        )],
    ));
}
