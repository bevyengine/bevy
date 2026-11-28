#[cfg(feature = "bevy_ci_testing")]
use bevy::{
    dev_tools::ci_testing::{CiTestingConfig, CiTestingEvent, CiTestingEventOnFrame},
    diagnostic::FrameCount,
    platform::collections::HashSet,
    prelude::*,
    render::view::screenshot::Captured,
    state::state::FreelyMutableState,
};

#[cfg(feature = "bevy_ci_testing")]
pub fn switch_scene_in_ci<Scene: States + FreelyMutableState + Next>(
    mut ci_config: ResMut<CiTestingConfig>,
    scene: Res<State<Scene>>,
    mut next_scene: ResMut<NextState<Scene>>,
    mut scenes_visited: Local<HashSet<Scene>>,
    frame_count: Res<FrameCount>,
    captured: RemovedComponents<Captured>,
) {
    if scene.is_changed() {
        // Changed scene! trigger a screenshot in 100 frames
        ci_config.events.push(CiTestingEventOnFrame(
            frame_count.0 + 100,
            CiTestingEvent::NamedScreenshot(format!("{:?}", scene.get())),
        ));
        return;
    }

    if !captured.is_empty() {
        // Screenshot taken! Switch to the next scene
        scenes_visited.insert(scene.get().clone());
        let next = scene.get().next();
        if scenes_visited.contains(&next) {
            // Exit once all scenes have been screenshotted
            ci_config.events.push(CiTestingEventOnFrame(
                frame_count.0 + 1,
                CiTestingEvent::AppExit,
            ));
        } else {
            next_scene.set(next);
        }
    }
}

pub trait Next {
    fn next(&self) -> Self;
}
