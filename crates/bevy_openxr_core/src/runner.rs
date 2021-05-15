use bevy_app::App;
use bevy_utils::Instant;

pub(crate) fn xr_runner(mut app: App) {
    let mut frame = 0;
    loop {
        let start = Instant::now();
        app.update();

        if frame % 70 == 0 {
            let took = start.elapsed();
            let fps = 1000.0 / took.as_millis() as f32;
            println!("Frame {} took {:?} ({} fps)", frame, took, fps);
        }

        frame += 1;
    }
}
