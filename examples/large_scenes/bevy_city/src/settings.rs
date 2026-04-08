use bevy::{
    camera::visibility::NoCpuCulling,
    camera_controller::free_camera::FreeCameraState,
    feathers::{
        self,
        controls::{button, checkbox, ButtonProps},
        theme::{ThemeBackgroundColor, ThemedText},
    },
    pbr::wireframe::WireframeConfig,
    prelude::*,
    ui::Checked,
    ui_widgets::{checkbox_self_update, Activate, ValueChange},
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
    pub cpu_culling: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            simulate_cars: true,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            wireframe_enabled: false,
            cpu_culling: true,
        }
    }
}

pub fn setup_settings_ui(mut commands: Commands) {
    commands.spawn_scene(settings_ui());
}

pub fn settings_ui() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(8.0)),
        }
        ThemeBackgroundColor(feathers::tokens::WINDOW_BG)
        on(|_: On<Pointer<Over>>, mut free_camera_state: Single<&mut FreeCameraState>| {
            free_camera_state.enabled = false;
        })
        on(|_: On<Pointer<Out>>, mut free_camera_state: Single<&mut FreeCameraState>| {
            free_camera_state.enabled = true;
        })
        Children [(
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                row_gap: px(8),
            }
            Children [
                Text("Settings"),
                (
                    checkbox()
                    Checked
                    on(checkbox_self_update)
                    on(|change: On<ValueChange<bool>>, mut settings: ResMut<Settings>| {
                        settings.simulate_cars = change.value;
                    })
                    Children [ (Text("Simulate Cars") ThemedText) ]
                ),
                (
                    checkbox()
                    Checked
                    on(checkbox_self_update)
                    on(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut directional_lights: Query<&mut DirectionalLight>| {
                            settings.shadow_maps_enabled = change.value;
                            for mut light in &mut directional_lights {
                                light.shadow_maps_enabled = change.value;

                            }
                        }
                    )
                    Children [ (Text("Shadow maps enabled") ThemedText) ]
                ),
                (
                    checkbox()
                    Checked
                    on(checkbox_self_update)
                    on(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut directional_lights: Query<&mut DirectionalLight>| {
                            settings.contact_shadows_enabled = change.value;
                            for mut light in &mut directional_lights {
                                light.contact_shadows_enabled = change.value;

                            }
                        }
                    )
                    Children [ (Text("Contact shadows enabled") ThemedText) ]
                ),
                (
                    checkbox()
                    on(checkbox_self_update)
                    on(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut wireframe_config: ResMut<WireframeConfig>| {
                            settings.wireframe_enabled = change.value;
                            wireframe_config.global = change.value;
                        }
                    )
                    Children [ (Text("Wireframe Enabled") ThemedText) ]
                ),
                (
                    checkbox()
                    Checked
                    on(checkbox_self_update)
                    on(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut commands: Commands,
                         meshes: Query<Entity, With<Mesh3d>>| {
                            settings.cpu_culling = change.value;

                            for entity in meshes.iter() {
                                if settings.cpu_culling {
                                    commands.entity(entity).remove::<NoCpuCulling>();
                                } else {
                                    commands.entity(entity).insert(NoCpuCulling);
                                }
                            }
                        }
                    )
                    Children [ (Text("CPU culling") ThemedText) ]
                ),
                (
                    button(ButtonProps::default())
                    on(
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
                    Children [ (Text("Regenerate City") ThemedText) ]
                ),
            ]
        )]
    }
}
