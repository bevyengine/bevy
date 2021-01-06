use crate::{app::AddAnimated, stage::ANIMATE};
use bevy_app::{App, AppBuilder};
use bevy_asset::{Asset, AssetServerSettings};
use bevy_ecs::{Component, Schedule};
use bevy_reflect::Struct;
use bevy_transform::prelude::*;

/// Bench utility
pub struct Bench {
    pub builder: AppBuilder,
    pub schedule: Schedule,
}

impl Bench {
    pub fn build(asset_folder: Option<String>) -> Self {
        let asset_folder = asset_folder.unwrap_or_else(|| {
            let mut asset_folder = std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            asset_folder.push_str("/assets");
            asset_folder
        });

        let mut builder = App::build();
        builder
            .add_resource(AssetServerSettings { asset_folder })
            .add_plugin(bevy_reflect::ReflectPlugin::default())
            .add_plugin(bevy_core::CorePlugin::default())
            .add_plugin(bevy_app::ScheduleRunnerPlugin::default())
            .add_plugin(bevy_asset::AssetPlugin::default())
            .add_plugin(TransformPlugin::default())
            .add_plugin(crate::AnimationPlugin {
                headless: true,
                ..Default::default()
            });

        let mut schedule = Schedule::default();
        schedule.add_stage(ANIMATE);
        schedule.add_system_to_stage(ANIMATE, crate::animator::animator_update_system);
        schedule.add_system_to_stage(
            ANIMATE,
            crate::reflect::animate_component_system::<Transform>,
        );

        Bench { builder, schedule }
    }

    pub fn warm(&mut self) {
        let app = &mut self.builder.app;
        app.initialize();
        app.update();
    }

    // fn register_animated_property_type<T: Lerp + Blend + Clone + 'static>(&mut self) -> &mut Self;

    pub fn register_animated_asset<T: Asset + Struct + Default>(&mut self) -> &mut Self {
        self.builder.register_animated_asset::<T>();
        self.schedule
            .add_system_to_stage(ANIMATE, crate::reflect::animate_asset_system::<T>);

        self
    }

    pub fn register_animated_component<T: Component + Struct + Default>(&mut self) -> &mut Self {
        self.builder.register_animated_component::<T>();
        self.schedule
            .add_system_to_stage(ANIMATE, crate::reflect::animate_component_system::<T>);

        self
    }

    /// Runs only animated related systems using a custom schedule
    pub fn run(&mut self, iterations: usize) -> &mut Self {
        let app = &mut self.builder.app;
        let world = &mut app.world;
        let resources = &mut app.resources;

        self.schedule.initialize(world, resources);
        for _ in 0..iterations {
            self.schedule.run(world, resources);
        }

        self
    }
}
