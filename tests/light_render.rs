//! Integration tests for the complete 2D lighting pipeline

use bevy::prelude::*;
use bevy::sprite::{FalloffType, PointLight2D};

/// Container for extracted 2D point lights ready for rendering
#[derive(Resource, Default)]
pub struct ExtractedPointLights2D(pub Vec<ExtractedPointLight2D>);

/// A 2D point light extracted from the world for rendering
#[derive(Clone)]
pub struct ExtractedPointLight2D {
    /// Position of the light in the world
    pub position: Vec2,
    /// Color of the light
    pub color: Color,
    /// Intensity of the light 0 means the light is off
    pub intensity: f32,
    /// Radius of the light
    pub radius: f32,
    /// falloff type, linear or exponential
    pub falloff: FalloffType,
}

fn extract_point_lights_2d(
    mut extracted_lights: ResMut<ExtractedPointLights2D>,
    light_query: Query<(&PointLight2D, &GlobalTransform)>,
) {
    extracted_lights.0.clear();

    for (light, transform) in light_query.iter() {
        extracted_lights.0.push(ExtractedPointLight2D {
            position: transform.translation().truncate(),
            color: light.color,
            intensity: light.intensity,
            radius: light.radius,
            falloff: light.falloff,
        });
    }
}

#[cfg(test)]
mod light_tests {
    use super::*;

    // Test that lights are properly extracted from main world to render world
    #[test]
    fn test_full_extraction_pipeline() {
        let mut app = App::new();

        // Add minimal plugins for testing
        app.add_plugins(MinimalPlugins).add_plugins(TransformPlugin);

        // Add our lighting systems
        app.add_systems(ExtractSchedule, extract_point_lights_2d);

        // Initialize resources
        app.insert_resource(ExtractedPointLights2D::default());

        // Spawn test entities in main world
        let _light_entity = app
            .world_mut()
            .spawn((
                PointLight2D {
                    color: Color::srgb(1.0, 1.0, 1.0),
                    intensity: 2.0,
                    radius: 150.0,
                    falloff: FalloffType::Exponential,
                },
                Transform::from_xyz(100.0, -50.0, 0.0),
                GlobalTransform::default(),
            ))
            .id();

        app.update();

        app.world_mut().run_schedule(ExtractSchedule);

        // Verify extraction worked
        let extracted = app.world().resource::<ExtractedPointLights2D>();
        assert_eq!(extracted.0.len(), 1);

        let extracted_light = &extracted.0[0];
        assert_eq!(extracted_light.position, Vec2::new(100.0, -50.0));
        assert_eq!(extracted_light.intensity, 2.0);
        assert_eq!(extracted_light.radius, 150.0);
        assert!(matches!(extracted_light.falloff, FalloffType::Exponential));
    }

    // Test resource initialization
    #[test]
    fn test_resource_initialization() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Resources should be properly initialized
        app.insert_resource(ExtractedPointLights2D::default());

        let extracted = app.world().resource::<ExtractedPointLights2D>();
        assert!(extracted.0.is_empty());

        app.update();
    }

    // Test component queries and filters
    #[test]
    fn test_light_queries() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(TransformPlugin);

        let _valid_light = app
            .world_mut()
            .spawn((
                PointLight2D {
                    color: Color::srgb(1.0, 0.0, 0.0),
                    intensity: 1.0,
                    radius: 100.0,
                    falloff: FalloffType::Linear,
                },
                Transform::default(),
                GlobalTransform::default(),
            ))
            .id();

        let _invalid_light = app
            .world_mut()
            .spawn((
                PointLight2D {
                    color: Color::srgb(0.0, 0.0, 1.0),
                    intensity: 1.0,
                    radius: 100.0,
                    falloff: FalloffType::Linear,
                },
                // Missing Transform/GlobalTransform
            ))
            .id();

        let _not_a_light = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                // Missing PointLight2D
            ))
            .id();

        app.update();

        // Query should only find valid lights
        let mut query = app.world_mut().query::<(&PointLight2D, &GlobalTransform)>();
        let results: Vec<_> = query.iter(app.world()).collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.color, Color::srgb(1.0, 0.0, 0.0));
    }

    // Test light modifications and updates
    #[test]
    fn test_light_updates() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(TransformPlugin);

        app.insert_resource(ExtractedPointLights2D::default());
        app.add_systems(ExtractSchedule, extract_point_lights_2d);

        let light_entity = app
            .world_mut()
            .spawn((
                PointLight2D {
                    color: Color::srgb(1.0, 1.0, 1.0),
                    intensity: 1.0,
                    radius: 100.0,
                    falloff: FalloffType::Linear,
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
                GlobalTransform::default(),
            ))
            .id();

        // Initial extraction
        app.update();
        app.world_mut().run_schedule(ExtractSchedule);

        {
            let extracted = app.world().resource::<ExtractedPointLights2D>();
            assert_eq!(extracted.0.len(), 1);
            assert_eq!(extracted.0[0].intensity, 1.0);
            assert_eq!(extracted.0[0].position, Vec2::ZERO);
        }

        // Modify light properties
        {
            let mut light = app
                .world_mut()
                .get_mut::<PointLight2D>(light_entity)
                .unwrap();
            light.intensity = 2.5;
            light.color = Color::srgb(1.0, 0.0, 0.0);
        }

        // Move the light
        {
            let mut transform = app.world_mut().get_mut::<Transform>(light_entity).unwrap();
            transform.translation = Vec3::new(50.0, 100.0, 0.0);
        }

        // Re-extract and verify changes
        app.update();
        app.world_mut().run_schedule(ExtractSchedule);

        {
            let extracted = app.world().resource::<ExtractedPointLights2D>();
            assert_eq!(extracted.0.len(), 1);
            assert_eq!(extracted.0[0].intensity, 2.5);
            assert_eq!(extracted.0[0].color, Color::srgb(1.0, 0.0, 0.0));
            assert_eq!(extracted.0[0].position, Vec2::new(50.0, 100.0));
        }
    }

    #[test]
    fn test_multiple_lights() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(TransformPlugin);

        app.insert_resource(ExtractedPointLights2D::default());
        app.add_systems(ExtractSchedule, extract_point_lights_2d);

        // Spawn multiple lights
        for i in 0..5 {
            app.world_mut().spawn((
                PointLight2D {
                    color: Color::srgb_from_array([
                        ((i as f32 * 72.0).to_radians().cos() + 1.0) / 2.0,
                        ((i as f32 * 72.0 + 120.0).to_radians().cos() + 1.0) / 2.0,
                        ((i as f32 * 72.0 + 240.0).to_radians().cos() + 1.0) / 2.0,
                    ]), // Different colors
                    intensity: (i + 1) as f32,
                    radius: 100.0 + i as f32 * 50.0,
                    falloff: if i % 2 == 0 {
                        FalloffType::Linear
                    } else {
                        FalloffType::Exponential
                    },
                },
                Transform::from_xyz(i as f32 * 100.0, 0.0, 0.0),
                GlobalTransform::default(),
            ));
        }

        app.update();
        app.world_mut().run_schedule(ExtractSchedule);

        let extracted = app.world().resource::<ExtractedPointLights2D>();
        assert_eq!(extracted.0.len(), 5);

        // Verify each light was extracted correctly
        for (i, light) in extracted.0.iter().enumerate() {
            assert_eq!(light.intensity, (i + 1) as f32);
            assert_eq!(light.position.x, i as f32 * 100.0);
            assert_eq!(light.radius, 100.0 + i as f32 * 50.0);
            // Test falloff pattern
            if i % 2 == 0 {
                assert!(matches!(light.falloff, FalloffType::Linear));
            } else {
                assert!(matches!(light.falloff, FalloffType::Exponential));
            }
        }
    }

    // Test light removal
    #[test]
    fn test_light_removal() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(TransformPlugin);

        app.insert_resource(ExtractedPointLights2D::default());
        app.add_systems(ExtractSchedule, extract_point_lights_2d);

        // Spawn lights
        let light1 = app
            .world_mut()
            .spawn((
                PointLight2D {
                    color: Color::srgb(1.0, 0.0, 0.0),
                    intensity: 1.0,
                    radius: 100.0,
                    falloff: FalloffType::Linear,
                },
                Transform::default(),
                GlobalTransform::default(),
            ))
            .id();

        let _light2 = app
            .world_mut()
            .spawn((
                PointLight2D {
                    color: Color::srgb(0.0, 0.0, 1.0),
                    intensity: 2.0,
                    radius: 150.0,
                    falloff: FalloffType::Exponential,
                },
                Transform::from_xyz(100.0, 0.0, 0.0),
                GlobalTransform::default(),
            ))
            .id();

        app.update();
        app.world_mut().run_schedule(ExtractSchedule);

        // Verify both lights exist
        {
            let extracted = app.world().resource::<ExtractedPointLights2D>();
            assert_eq!(extracted.0.len(), 2);
        }

        // Remove one light
        app.world_mut().despawn(light1);

        app.update();
        app.world_mut().run_schedule(ExtractSchedule);

        // Verify only one light remains
        {
            let extracted = app.world().resource::<ExtractedPointLights2D>();
            assert_eq!(extracted.0.len(), 1);
            assert_eq!(extracted.0[0].color, Color::srgb(0.0, 0.0, 1.0));
            assert_eq!(extracted.0[0].intensity, 2.0);
        }
    }

    // Test performance with many lights
    #[test]
    fn test_many_lights_performance() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(TransformPlugin);

        app.insert_resource(ExtractedPointLights2D::default());
        app.add_systems(ExtractSchedule, extract_point_lights_2d);

        // Spawn many lights
        const LIGHT_COUNT: usize = 1000;
        for i in 0..LIGHT_COUNT {
            app.world_mut().spawn((
                PointLight2D {
                    color: Color::WHITE,
                    intensity: 1.0,
                    radius: 100.0,
                    falloff: FalloffType::Linear,
                },
                Transform::from_xyz((i % 100) as f32 * 10.0, (i / 100) as f32 * 10.0, 0.0),
                GlobalTransform::default(),
            ));
        }

        let start = std::time::Instant::now();
        app.update();
        app.world_mut().run_schedule(ExtractSchedule);
        let duration = start.elapsed();

        let extracted = app.world().resource::<ExtractedPointLights2D>();
        assert_eq!(extracted.0.len(), LIGHT_COUNT);

        assert!(
            duration.as_millis() < 100,
            "Extraction took too long: {:?}",
            duration
        );
    }
}
